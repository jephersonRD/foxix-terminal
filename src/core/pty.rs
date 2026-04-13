use anyhow::Result;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

pub struct PtyMaster {
    fd: OwnedFd,
    pts_name: String,
}

impl PtyMaster {
    pub fn open() -> Result<Self> {
        use libc::{grantpt, ptsname_r, unlockpt, O_NOCTTY, O_RDWR};

        let master_fd = unsafe { libc::posix_openpt(O_RDWR | O_NOCTTY | libc::O_NONBLOCK) };
        if master_fd < 0 {
            anyhow::bail!("Failed to open PTY master");
        }

        log::info!("PTY master opened with fd: {}", master_fd);

        let grant_result = unsafe { grantpt(master_fd) };
        log::info!("grantpt result: {}", grant_result);
        if grant_result != 0 {
            let errno = std::io::Error::last_os_error();
            log::error!("grantpt failed: {}", errno);
            unsafe { libc::close(master_fd) };
            anyhow::bail!("Failed to grant PTY: {}", errno);
        }

        let unlock_result = unsafe { unlockpt(master_fd) };
        log::info!("unlockpt result: {}", unlock_result);
        if unlock_result != 0 {
            let errno = std::io::Error::last_os_error();
            log::error!("unlockpt failed: {}", errno);
            unsafe { libc::close(master_fd) };
            anyhow::bail!("Failed to unlock PTY: {}", errno);
        }

        let mut pts_name = vec![0u8; 128];
        let result =
            unsafe { ptsname_r(master_fd, pts_name.as_mut_ptr() as *mut libc::c_char, 128) };
        if result != 0 {
            unsafe { libc::close(master_fd) };
            anyhow::bail!("Failed to get PTY name: {}", result);
        }
        let null_pos = pts_name.iter().position(|&c| c == 0).unwrap_or(0);
        let pts_name = String::from_utf8_lossy(&pts_name[..null_pos]).to_string();

        log::info!("Got PTS name: {}", pts_name);

        use std::fs;
        match fs::metadata(&pts_name) {
            Ok(meta) => log::info!("PTS metadata: {:?}", meta),
            Err(e) => log::warn!("PTS metadata error: {}", e),
        }

        Ok(Self {
            fd: unsafe { OwnedFd::from_raw_fd(master_fd) },
            pts_name,
        })
    }

    pub fn pts_name(&self) -> &str {
        &self.pts_name
    }

    pub fn fd(&self) -> &OwnedFd {
        &self.fd
    }

    pub fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.fd.as_raw_fd()
    }

    pub fn open_slave(&self) -> Result<File> {
        use libc::{open, O_NOCTTY, O_RDWR};

        log::info!("Opening PTY slave: {}", self.pts_name);

        let cstr = std::ffi::CString::new(self.pts_name.as_bytes())
            .map_err(|_| anyhow::anyhow!("Failed to create CString"))?;

        use std::fs;
        if let Ok(metadata) = fs::metadata(&self.pts_name) {
            log::info!("PTS exists, is_file: {}", metadata.is_file());
        } else {
            log::warn!("PTS does NOT exist!");
        }

        let slave_fd = unsafe { open(cstr.as_ptr(), O_RDWR | O_NOCTTY) };

        if slave_fd < 0 {
            let errno = std::io::Error::last_os_error();
            log::error!("Failed to open PTY slave '{}': {}", self.pts_name, errno);
            anyhow::bail!("Failed to open PTY slave: {}", errno);
        }

        log::info!("PTY slave opened successfully");
        Ok(unsafe { File::from_raw_fd(slave_fd) })
    }
}
