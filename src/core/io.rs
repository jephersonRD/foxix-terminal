use libc::read;
use std::collections::VecDeque;

pub struct AsyncIOHandler {
    buffer: VecDeque<u8>,
    max_buffer_size: usize,
}

impl AsyncIOHandler {
    pub fn new(max_buffer_size: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_buffer_size),
            max_buffer_size,
        }
    }

    pub fn read_available<F>(&mut self, fd: i32, mut callback: F) -> Result<bool, std::io::Error>
    where
        F: FnMut(&[u8]),
    {
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe { read(fd, buf.as_mut_ptr() as *mut _, buf.len()) };
            if n < 0 {
                let e = std::io::Error::last_os_error();
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    break;
                }
                return Err(e);
            }
            if n == 0 {
                break;
            }

            if self.buffer.len() + n as usize > self.max_buffer_size {
                let overflow = (self.buffer.len() + n as usize) - self.max_buffer_size;
                self.buffer.drain(..overflow);
            }
            self.buffer.extend(&buf[..n as usize]);
            callback(&self.buffer.make_contiguous());
        }
        Ok(true)
    }

    pub fn flush_to(&mut self, target: &mut Vec<u8>, max_bytes: usize) {
        let count = std::cmp::min(self.buffer.len(), max_bytes);
        for _ in 0..count {
            if let Some(byte) = self.buffer.pop_front() {
                target.push(byte);
            }
        }
    }

    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}
