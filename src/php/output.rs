//! PHP Output Buffering
//!
//! Migrated from main/php_output.h and main/output.c

#[cfg(test)]
mod tests;

/// Output buffer
pub struct OutputBuffer {
    buffer: Vec<u8>,
    level: usize, // Used for nested output buffering
}

impl OutputBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            level: 0,
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize, String> {
        self.buffer.extend_from_slice(data);
        Ok(data.len())
    }

    pub fn write_str(&mut self, s: &str) -> Result<usize, String> {
        self.write(s.as_bytes())
    }

    pub fn flush(&mut self) -> Result<(), String> {
        // TODO: Implement flush to output
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

/// Output buffer stack (thread-safe)
static OUTPUT_BUFFERS: std::sync::Mutex<Vec<OutputBuffer>> = std::sync::Mutex::new(Vec::new());

/// Start output buffering
pub fn php_output_start() -> Result<(), String> {
    let mut buffers = OUTPUT_BUFFERS.lock().map_err(|e| e.to_string())?;
    buffers.push(OutputBuffer::new());
    Ok(())
}

/// End output buffering and get contents
pub fn php_output_end() -> Result<String, String> {
    let mut buffers = OUTPUT_BUFFERS.lock().map_err(|e| e.to_string())?;
    if let Some(buffer) = buffers.pop() {
        Ok(buffer.get_contents_string())
    } else {
        Err("No output buffer to end".to_string())
    }
}

/// Write to current output buffer
pub fn php_output_write(data: &[u8]) -> Result<usize, String> {
    let mut buffers = OUTPUT_BUFFERS.lock().map_err(|e| e.to_string())?;
    if let Some(buffer) = buffers.last_mut() {
        buffer.write(data)
    } else {
        // No buffer, write directly (would go to stdout in real implementation)
        Ok(data.len())
    }
}

/// Write string to output
pub fn php_printf(format: &str, args: &[&str]) -> Result<(), String> {
    // Simple printf implementation
    let mut result = format.to_string();
    for (i, arg) in args.iter().enumerate() {
        result = result.replace(&format!("{{{i}}}"), arg);
    }
    php_output_write(result.as_bytes())?;
    Ok(())
}
