use anyhow::Result;
use std::ffi::CString;
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct ChildProcess {
    pid: libc::pid_t,
    master_fd: OwnedFd,
    should_exit: Arc<AtomicBool>,
}

impl ChildProcess {
    pub fn spawn(
        shell: &str,
        master: crate::core::pty::PtyMaster,
        rows: u16,
        cols: u16,
    ) -> Result<Self> {
        let master_raw = master.as_raw_fd();
        let slave_file = master.open_slave()?;
        let slave_raw = slave_file.as_raw_fd();

        // Configurar tamaño de ventana antes del fork
        unsafe {
            let ws = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            libc::ioctl(master_raw, libc::TIOCSWINSZ, &ws);
        }

        let should_exit = Arc::new(AtomicBool::new(false));

        let pid = unsafe { libc::fork() };
        if pid < 0 {
            anyhow::bail!("Failed to fork: {}", std::io::Error::last_os_error());
        }

        if pid == 0 {
            // ── PROCESO HIJO ──────────────────────────────────────────────
            unsafe {
                // 1. Nueva sesión (debe ser lo primero)
                if libc::setsid() < 0 {
                    libc::_exit(1);
                }

                // 2. Asignar el slave como terminal de control
                if libc::ioctl(slave_raw, libc::TIOCSCTTY, 0i32) < 0 {
                    // Algunos kernels no requieren esto, continuar igual
                }

                // 3. Redirigir stdin/stdout/stderr al slave PTY
                libc::dup2(slave_raw, 0);
                libc::dup2(slave_raw, 1);
                libc::dup2(slave_raw, 2);

                // 4. Cerrar el slave FD extra si no es 0/1/2
                if slave_raw > 2 {
                    libc::close(slave_raw);
                }

                // 5. Cerrar el master FD en el hijo (MUY IMPORTANTE)
                libc::close(master_raw);

                // 6. Configurar variables de entorno del terminal
                // TERM_PROGRAM=kitty + KITTY_WINDOW_ID activa el Kitty Graphics Protocol
                // en yazi, timg, kitten icat, btop, etc.
                let envs: &[&str] = &[
                    "TERM=xterm-kitty",
                    "TERM_PROGRAM=kitty",
                    "TERM_PROGRAM_VERSION=0.40.0",
                    "KITTY_WINDOW_ID=1",
                    "COLORTERM=truecolor",
                    "FOXIX=1",
                ];
                for env in envs {
                    if let Ok(cs) = CString::new(*env) {
                        libc::putenv(cs.as_ptr() as *mut libc::c_char);
                        // NOTE: putenv en el hijo — el CString no se libera (se hace _exit después)
                        std::mem::forget(cs);
                    }
                }

                // 7. Ejecutar el shell con execvp (REEMPLAZA el proceso, no crea uno nuevo)
                let shell_cstr = CString::new(shell).unwrap_or_else(|_| CString::new("/bin/bash").unwrap());
                // argv[0] = nombre del shell (con '-' para login shell)
                let argv0 = {
                    let name = std::path::Path::new(shell)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("bash");
                    // Prefijo '-' indica login shell (como hace la mayoría de terminales)
                    let login_name = format!("-{}", name);
                    CString::new(login_name).unwrap()
                };
                let args: [*const libc::c_char; 2] = [
                    argv0.as_ptr(),
                    std::ptr::null(),
                ];
                libc::execvp(shell_cstr.as_ptr(), args.as_ptr());

                // Si execvp falla, salir
                libc::_exit(127);
            }
        }

        // ── PROCESO PADRE ──────────────────────────────────────────────────
        // El slave_file se drop aquí (el padre no necesita el slave)
        drop(slave_file);

        // Clonar el master FD para el struct
        let master_fd = master.fd().try_clone()?;

        log::info!("Shell '{}' lanzado con PID: {}", shell, pid);

        Ok(Self {
            pid,
            master_fd,
            should_exit,
        })
    }

    pub fn pid(&self) -> libc::pid_t {
        self.pid
    }

    pub fn write(&self, data: &[u8]) -> Result<usize> {
        let n = unsafe {
            libc::write(
                self.master_fd.as_raw_fd(),
                data.as_ptr() as *const _,
                data.len(),
            )
        };
        if n < 0 {
            let e = std::io::Error::last_os_error();
            // EAGAIN es normal con FD no-bloqueante
            if e.kind() == std::io::ErrorKind::WouldBlock {
                return Ok(0);
            }
            anyhow::bail!("Write to PTY failed: {}", e);
        }
        Ok(n as usize)
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        let n = unsafe {
            libc::read(
                self.master_fd.as_raw_fd(),
                buf.as_mut_ptr() as *mut _,
                buf.len(),
            )
        };
        if n < 0 {
            let e = std::io::Error::last_os_error();
            if e.kind() == std::io::ErrorKind::WouldBlock {
                return Ok(0);
            }
            anyhow::bail!("Read from PTY failed: {}", e);
        }
        Ok(n as usize)
    }

    pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        unsafe {
            libc::ioctl(self.master_fd.as_raw_fd(), libc::TIOCSWINSZ, &ws);
            // Notificar al proceso del cambio de tamaño
            libc::kill(self.pid, libc::SIGWINCH);
        }
        Ok(())
    }

    pub fn is_alive(&self) -> bool {
        unsafe { libc::kill(self.pid, 0) == 0 }
    }

    pub fn kill(&self) -> Result<()> {
        unsafe {
            libc::kill(self.pid, libc::SIGTERM);
            // Esperar un poco y forzar si no muere
            std::thread::sleep(std::time::Duration::from_millis(100));
            libc::kill(self.pid, libc::SIGKILL);
        }
        Ok(())
    }

    pub fn master_fd(&self) -> &OwnedFd {
        &self.master_fd
    }

    pub fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.master_fd.as_raw_fd()
    }
}

impl Drop for ChildProcess {
    fn drop(&mut self) {
        self.kill().ok();
        // Esperar al hijo para no crear zombis
        unsafe {
            let mut status: libc::c_int = 0;
            libc::waitpid(self.pid, &mut status, libc::WNOHANG);
        }
    }
}
