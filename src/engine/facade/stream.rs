//! Stream Facade - Enhanced stream operations
//!
//! Provides builder pattern and higher-level stream operations

use crate::php::streams::{FileStream, StreamError, StreamMode, StreamResult};

/// Builder for creating streams with fluent API
pub struct StreamBuilder {
    path: Option<String>,
    mode: StreamMode,
    buffered: bool,
}

impl StreamBuilder {
    /// Create a new stream builder
    pub fn new() -> Self {
        Self {
            path: None,
            mode: StreamMode::Read,
            buffered: false,
        }
    }

    /// Set the path for the stream
    pub fn path(mut self, path: &str) -> Self {
        self.path = Some(path.to_string());
        self
    }

    /// Set the mode for the stream
    pub fn mode(mut self, mode: StreamMode) -> Self {
        self.mode = mode;
        self
    }

    /// Enable buffering
    pub fn buffered(mut self, buffered: bool) -> Self {
        self.buffered = buffered;
        self
    }

    /// Build the stream
    pub fn build(self) -> StreamResult<FileStream> {
        let path = self.path.ok_or_else(|| StreamError::InvalidOperation)?;
        FileStream::open(&path, self.mode)
    }

    /// Open read-only
    pub fn read(path: &str) -> StreamResult<FileStream> {
        Self::new().path(path).mode(StreamMode::Read).build()
    }

    /// Open write-only (truncate)
    pub fn write(path: &str) -> StreamResult<FileStream> {
        Self::new().path(path).mode(StreamMode::Write).build()
    }

    /// Open append
    pub fn append(path: &str) -> StreamResult<FileStream> {
        Self::new().path(path).mode(StreamMode::Append).build()
    }

    /// Open read-write
    pub fn read_write(path: &str) -> StreamResult<FileStream> {
        Self::new().path(path).mode(StreamMode::ReadWrite).build()
    }
}

impl Default for StreamBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper for reading entire files
pub fn read_file_contents(path: &str) -> StreamResult<String> {
    let mut stream = StreamBuilder::read(path)?;
    let mut contents = String::new();
    use std::io::Read;
    stream.read_to_string(&mut contents)
        .map_err(|e| StreamError::IoError(e.to_string()))?;
    Ok(contents)
}

/// Helper for writing entire files
pub fn write_file_contents(path: &str, contents: &str) -> StreamResult<()> {
    let mut stream = StreamBuilder::write(path)?;
    use std::io::Write;
    stream.write_all(contents.as_bytes())
        .map_err(|e| StreamError::IoError(e.to_string()))?;
    stream.flush()
        .map_err(|e| StreamError::IoError(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_stream_builder_read() {
        // Create a test file
        let test_path = "/tmp/test_stream_read.txt";
        let test_content = "Hello, World!";
        fs::write(test_path, test_content).unwrap();

        let result = StreamBuilder::read(test_path);
        assert!(result.is_ok());

        // Cleanup
        let _ = fs::remove_file(test_path);
    }

    #[test]
    fn test_stream_builder_write() {
        let test_path = "/tmp/test_stream_write.txt";

        let result = StreamBuilder::write(test_path);
        assert!(result.is_ok());

        // Cleanup
        let _ = fs::remove_file(test_path);
    }

    #[test]
    fn test_read_write_helpers() {
        let test_path = "/tmp/test_stream_helpers.txt";
        let test_content = "Test content for helpers";

        // Write
        let write_result = write_file_contents(test_path, test_content);
        assert!(write_result.is_ok());

        // Read
        let read_result = read_file_contents(test_path);
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), test_content);

        // Cleanup
        let _ = fs::remove_file(test_path);
    }
}
