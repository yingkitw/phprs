//! PHP Output Buffering
//!
//! Migrated from main/php_output.h and main/output.c
//! Supports multiple buffer levels, ob_start/ob_end_clean/ob_end_flush,
//! ob_get_contents/ob_get_clean/ob_get_flush/ob_get_level.

#[cfg(test)]
mod tests;

pub struct OutputBuffer {
    buffer: Vec<u8>,
}

impl OutputBuffer {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize, String> {
        self.buffer.extend_from_slice(data);
        Ok(data.len())
    }

    pub fn write_str(&mut self, s: &str) -> Result<usize, String> {
        self.write(s.as_bytes())
    }

    pub fn flush(&mut self) -> Result<(), String> {
        use std::io::Write;
        std::io::stdout()
            .write_all(&self.buffer)
            .map_err(|e| e.to_string())?;
        std::io::stdout().flush().map_err(|e| e.to_string())?;
        self.buffer.clear();
        Ok(())
    }

    pub fn get_contents(&self) -> &[u8] {
        &self.buffer
    }

    pub fn get_contents_string(&self) -> String {
        String::from_utf8_lossy(&self.buffer).to_string()
    }

    pub fn clean(&mut self) {
        self.buffer.clear();
    }
}

impl Default for OutputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    static OUTPUT_BUFFERS: std::cell::RefCell<Vec<OutputBuffer>> = std::cell::RefCell::new(Vec::new());
}

/// Start output buffering
pub fn php_output_start() -> Result<(), String> {
    OUTPUT_BUFFERS.with(|buffers| {
        buffers.borrow_mut().push(OutputBuffer::new());
        Ok(())
    })
}

/// End output buffering and get contents
pub fn php_output_end() -> Result<String, String> {
    OUTPUT_BUFFERS.with(|buffers| {
        let mut bufs = buffers.borrow_mut();
        if let Some(buffer) = bufs.pop() {
            Ok(buffer.get_contents_string())
        } else {
            Err("No output buffer to end".to_string())
        }
    })
}

/// End output buffering, flush contents to stdout (or parent buffer), return contents
pub fn php_output_end_flush() -> Result<String, String> {
    OUTPUT_BUFFERS.with(|buffers| {
        let mut bufs = buffers.borrow_mut();
        if let Some(buffer) = bufs.pop() {
            let contents = buffer.get_contents_string();
            if !bufs.is_empty() {
                let _ = bufs.last_mut().unwrap().write(contents.as_bytes());
            } else {
                let _ = std::io::Write::write_all(&mut std::io::stdout(), contents.as_bytes());
            }
            Ok(contents)
        } else {
            Err("No output buffer to end".to_string())
        }
    })
}

/// End output buffering and discard contents
pub fn php_output_end_clean() -> Result<(), String> {
    OUTPUT_BUFFERS.with(|buffers| {
        let mut bufs = buffers.borrow_mut();
        if bufs.pop().is_some() {
            Ok(())
        } else {
            Err("No output buffer to end".to_string())
        }
    })
}

/// Get current buffer level (number of active buffers)
pub fn php_output_get_level() -> usize {
    OUTPUT_BUFFERS.with(|buffers| buffers.borrow().len())
}

/// Get contents of current buffer without ending it
pub fn php_output_get_contents() -> Result<String, String> {
    OUTPUT_BUFFERS.with(|buffers| {
        let bufs = buffers.borrow();
        if let Some(buffer) = bufs.last() {
            Ok(buffer.get_contents_string())
        } else {
            Ok(String::new())
        }
    })
}

/// Get contents and clean the current buffer (without ending it)
pub fn php_output_get_clean() -> Result<String, String> {
    OUTPUT_BUFFERS.with(|buffers| {
        let mut bufs = buffers.borrow_mut();
        if let Some(buffer) = bufs.last_mut() {
            let contents = buffer.get_contents_string();
            buffer.clean();
            Ok(contents)
        } else {
            Ok(String::new())
        }
    })
}

/// Get contents, flush to stdout/parent, and clean the current buffer
pub fn php_output_get_flush() -> Result<String, String> {
    OUTPUT_BUFFERS.with(|buffers| {
        let mut bufs = buffers.borrow_mut();
        let len = bufs.len();
        if len == 0 {
            return Ok(String::new());
        }
        let contents = bufs.last().unwrap().get_contents_string();
        bufs.last_mut().unwrap().clean();
        if len > 1 {
            let content_bytes = contents.as_bytes().to_vec();
            bufs[len - 2].write(&content_bytes)?;
        } else {
            let _ = std::io::Write::write_all(&mut std::io::stdout(), contents.as_bytes());
        }
        Ok(contents)
    })
}

/// Clean (erase) the current output buffer
pub fn php_output_clean() -> Result<(), String> {
    OUTPUT_BUFFERS.with(|buffers| {
        let mut bufs = buffers.borrow_mut();
        if let Some(buffer) = bufs.last_mut() {
            buffer.clean();
            Ok(())
        } else {
            Err("No output buffer to clean".to_string())
        }
    })
}

/// Flush the current output buffer (send to stdout/parent)
pub fn php_output_flush() -> Result<(), String> {
    OUTPUT_BUFFERS.with(|buffers| {
        let mut bufs = buffers.borrow_mut();
        let len = bufs.len();
        if len == 0 {
            return Err("No output buffer to flush".to_string());
        }
        let contents = bufs.last().unwrap().get_contents_string();
        bufs.last_mut().unwrap().clean();
        if len > 1 {
            let content_bytes = contents.as_bytes().to_vec();
            bufs[len - 2].write(&content_bytes)?;
        } else {
            let _ = std::io::Write::write_all(&mut std::io::stdout(), contents.as_bytes());
        }
        Ok(())
    })
}

/// Write to current output buffer
pub fn php_output_write(data: &[u8]) -> Result<usize, String> {
    OUTPUT_BUFFERS.with(|buffers| {
        let mut bufs = buffers.borrow_mut();
        if let Some(buffer) = bufs.last_mut() {
            buffer.write(data)
        } else {
            Ok(data.len())
        }
    })
}

/// Write string to output
pub fn php_printf(format: &str, args: &[&str]) -> Result<(), String> {
    let mut result = format.to_string();
    for (i, arg) in args.iter().enumerate() {
        result = result.replace(&format!("{{{i}}}"), arg);
    }
    php_output_write(result.as_bytes())?;
    Ok(())
}
