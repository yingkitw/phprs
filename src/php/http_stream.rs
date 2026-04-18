//! HTTP Stream Wrapper
//!
//! Implements HTTP/HTTPS stream support for fopen, file_get_contents, etc.

use std::io::{Read, Seek, SeekFrom, Write};

/// HTTP Stream for reading from URLs
pub struct HttpStream {
    #[allow(dead_code)]
    url: String,
    content: Vec<u8>,
    position: usize,
}

impl HttpStream {
    /// Open an HTTP/HTTPS URL
    pub fn open(url: &str) -> Result<Self, String> {
        // Use reqwest to fetch the content synchronously
        let content = Self::fetch_url(url)?;
        
        Ok(Self {
            url: url.to_string(),
            content,
            position: 0,
        })
    }
    
    /// Fetch URL content (blocking)
    fn fetch_url(url: &str) -> Result<Vec<u8>, String> {
        // Create a runtime for blocking operation
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;
        
        rt.block_on(async {
            let response = reqwest::get(url)
                .await
                .map_err(|e| format!("HTTP request failed: {}", e))?;
            
            if !response.status().is_success() {
                return Err(format!("HTTP error: {}", response.status()));
            }
            
            response.bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(|e| format!("Failed to read response: {}", e))
        })
    }
    
    /// Get content as string
    pub fn get_contents(&self) -> String {
        String::from_utf8_lossy(&self.content).to_string()
    }
}

impl Read for HttpStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.position >= self.content.len() {
            return Ok(0);
        }
        
        let remaining = &self.content[self.position..];
        let to_read = remaining.len().min(buf.len());
        buf[..to_read].copy_from_slice(&remaining[..to_read]);
        self.position += to_read;
        Ok(to_read)
    }
}

impl Write for HttpStream {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "HTTP streams are read-only"
        ))
    }
    
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for HttpStream {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as usize,
            SeekFrom::End(offset) => {
                if offset >= 0 {
                    self.content.len() + offset as usize
                } else {
                    self.content.len().saturating_sub((-offset) as usize)
                }
            }
            SeekFrom::Current(offset) => {
                if offset >= 0 {
                    self.position + offset as usize
                } else {
                    self.position.saturating_sub((-offset) as usize)
                }
            }
        };
        
        self.position = new_pos.min(self.content.len());
        Ok(self.position as u64)
    }
}

/// Fetch URL content as string (convenience function)
pub fn file_get_contents_http(url: &str) -> Result<String, String> {
    let stream = HttpStream::open(url)?;
    Ok(stream.get_contents())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_http_stream_creation() {
        // This test would require network access
        // In real tests, you'd use a mock HTTP server
    }
}
