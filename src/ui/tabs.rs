use anyhow::Result;
use libc::read;
use std::os::fd::AsRawFd;

use crate::core::io::AsyncIOHandler;
use crate::core::process::ChildProcess;
use crate::core::pty::PtyMaster;
use crate::terminal::AnsiParser;

pub struct Tab {
    pub id: usize,
    pub title: String,
    pub shell: String,
    pub rows: usize,
    pub cols: usize,
    process: Option<ChildProcess>,
    pub parser: AnsiParser,
    io_handler: AsyncIOHandler,
}

impl Tab {
    pub fn new(id: usize, shell: String, rows: usize, cols: usize) -> Result<Self> {
        log::info!(
            "Creating tab {} with shell: {} ({}x{})",
            id,
            shell,
            cols,
            rows
        );
        let pty = PtyMaster::open()?;
        log::info!("PTY opened: {}", pty.pts_name());
        let process = ChildProcess::spawn(&shell, pty, rows as u16, cols as u16)?;
        log::info!("Process spawned with PID: {}", process.pid());

        Ok(Self {
            id,
            title: shell.clone(),
            shell,
            rows,
            cols,
            process: Some(process),
            parser: AnsiParser::new(rows, cols),
            io_handler: AsyncIOHandler::new(65536),
        })
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        // Solo actuar si el tamaño realmente cambió
        if self.rows == rows && self.cols == cols {
            return;
        }
        let old_cols = self.cols;
        let old_rows = self.rows;
        self.rows = rows;
        self.cols = cols;

        // 1. Notificar al proceso (TIOCSWINSZ + SIGWINCH) — esto hace que el shell
        //    re-dibuje su UI usando el nuevo tamaño. El shell es responsable del redraw.
        if let Some(ref process) = self.process {
            process.resize(rows as u16, cols as u16).ok();
        }

        // 2. Resize del parser/buffer — preserva el contenido existente
        self.parser.resize(rows, cols);

        log::info!("Tab resize: {}x{} -> {}x{}", old_cols, old_rows, cols, rows);
    }

    pub fn write_input(&mut self, data: &[u8]) -> Result<usize> {
        if let Some(ref process) = self.process {
            Ok(process.write(data)?)
        } else {
            Ok(0)
        }
    }

    pub fn read_output(&mut self) -> Result<bool> {
        if let Some(ref process) = self.process {
            if !process.is_alive() {
                log::warn!("Process {} is not alive", process.pid());
                return Ok(false);
            }

            let fd = process.as_raw_fd();
            let mut buf = [0u8; 4096];
            let mut got_data = false;

            loop {
                let n = unsafe { read(fd, buf.as_mut_ptr() as *mut _, buf.len()) };
                if n < 0 {
                    let e = std::io::Error::last_os_error();
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        break;
                    }
                    log::warn!("PTY read error: {}", e);
                    break;
                }
                if n == 0 {
                    break;
                }
                log::debug!("Read {} bytes from PTY", n);
                self.parser.parse(&buf[..n as usize]);
                got_data = true;
            }

            // ── Drenar respuestas pendientes (DA1/DA2/CPR) ─────────────────
            // El parser acumula respuestas que el shell espera del terminal.
            // Las escribimos de vuelta al PTY master inmediatamente.
            let responses: Vec<Vec<u8>> = self.parser.pending_responses.drain(..).collect();
            for resp in responses {
                process.write(&resp).ok();
            }

            Ok(got_data)
        } else {
            Ok(false)
        }
    }

    pub fn is_alive(&self) -> bool {
        self.process.as_ref().map(|p| p.is_alive()).unwrap_or(false)
    }

    pub fn close(&mut self) {
        if let Some(process) = self.process.take() {
            process.kill().ok();
        }
    }
}

pub struct TabManager {
    tabs: Vec<Tab>,
    active_tab: usize,
    next_id: usize,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
            next_id: 1,
        }
    }

    pub fn create_tab(&mut self, shell: String, rows: usize, cols: usize) -> Result<usize> {
        let tab = Tab::new(self.next_id, shell, rows, cols)?;
        let id = tab.id;
        self.next_id += 1;
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        Ok(id)
    }

    pub fn close_tab(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            self.tabs[index].close();
            self.tabs.remove(index);
            if self.tabs.is_empty() {
                return false;
            }
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
            true
        } else {
            true
        }
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab)
    }

    pub fn set_active_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = index;
        }
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
        }
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn resize_all(&mut self, rows: usize, cols: usize) {
        for tab in &mut self.tabs {
            tab.resize(rows, cols);
        }
    }

    pub fn close_active(&mut self) -> bool {
        self.close_tab(self.active_tab)
    }
}
