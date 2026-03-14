//! PHP Streams
//!
//! Stream abstraction layer
//! Migrated from main/php_streams.h and main/streams/

#[cfg(test)]
mod tests;

use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;

/// Stream operations result
pub type StreamResult<T> = Result<T, StreamError>;

/// Stream errors
#[derive(Debug)]
pub enum StreamError {
    NotFound,
    PermissionDenied,
    IoError(String),
    InvalidOperation,
}

/// Stream mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamMode {
    Read,
    Write,
    Append,
    ReadWrite,
}

/// Stream wrapper trait
pub trait StreamWrapper: Read + Write + Seek {
    fn url_stat(&self) -> StreamResult<StreamStat>;
    fn unlink(&mut self) -> StreamResult<()>;
    fn rename(&mut self, new_path: &str) -> StreamResult<()>;
}

/// Stream statistics
#[derive(Debug, Clone)]
pub struct StreamStat {
    pub size: u64,
    pub mode: u32,
    pub mtime: i64,
    pub ctime: i64,
}

/// File stream implementation
pub struct FileStream {
    file: File,
    path: String,
}

impl FileStream {
    pub fn open(path: &str, mode: StreamMode) -> StreamResult<Self> {
        use std::fs::OpenOptions;

        let mut opts = OpenOptions::new();
        match mode {
            StreamMode::Read => opts.read(true),
            StreamMode::Write => opts.write(true).create(true).truncate(true),
            StreamMode::Append => opts.write(true).create(true).append(true),
            StreamMode::ReadWrite => opts.read(true).write(true).create(true),
        };

        let file = opts
            .open(path)
            .map_err(|e| StreamError::IoError(e.to_string()))?;

        Ok(Self {
            file,
            path: path.to_string(),
        })
    }
}

impl Read for FileStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Write for FileStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl Seek for FileStream {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl StreamWrapper for FileStream {
    fn url_stat(&self) -> StreamResult<StreamStat> {
        let metadata =
            std::fs::metadata(&self.path).map_err(|e| StreamError::IoError(e.to_string()))?;

        let file_type = metadata.file_type();
        let mode = if file_type.is_file() {
            0o100000
        } else if file_type.is_dir() {
            0o040000
        } else {
            0o120000
        };

        Ok(StreamStat {
            size: metadata.len(),
            mode,
            mtime: metadata
                .modified()
                .and_then(|t| Ok(t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64))
                .unwrap_or(0),
            ctime: metadata
                .created()
                .and_then(|t| Ok(t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64))
                .unwrap_or(0),
        })
    }

    fn unlink(&mut self) -> StreamResult<()> {
        std::fs::remove_file(&self.path).map_err(|e| StreamError::IoError(e.to_string()))?;
        Ok(())
    }

    fn rename(&mut self, new_path: &str) -> StreamResult<()> {
        std::fs::rename(&self.path, new_path).map_err(|e| StreamError::IoError(e.to_string()))?;
        self.path = new_path.to_string();
        Ok(())
    }
}

/// Open a stream
pub fn php_stream_open(path: &str, mode: StreamMode) -> StreamResult<FileStream> {
    if Path::new(path).exists() || mode != StreamMode::Read {
        FileStream::open(path, mode)
    } else {
        Err(StreamError::NotFound)
    }
}

// ============================================================
// Stream Filters
// ============================================================

/// Stream filter trait - processes data as it passes through a stream
pub trait StreamFilter: Send {
    /// Filter name (e.g. "string.toupper")
    fn name(&self) -> &str;

    /// Filter data on read (from stream to consumer)
    fn filter_read(&self, data: &[u8]) -> Vec<u8>;

    /// Filter data on write (from producer to stream)
    fn filter_write(&self, data: &[u8]) -> Vec<u8>;
}

/// Built-in string.toupper filter
pub struct StringToUpperFilter;

impl StreamFilter for StringToUpperFilter {
    fn name(&self) -> &str {
        "string.toupper"
    }

    fn filter_read(&self, data: &[u8]) -> Vec<u8> {
        data.iter().map(|b| b.to_ascii_uppercase()).collect()
    }

    fn filter_write(&self, data: &[u8]) -> Vec<u8> {
        data.iter().map(|b| b.to_ascii_uppercase()).collect()
    }
}

/// Built-in string.tolower filter
pub struct StringToLowerFilter;

impl StreamFilter for StringToLowerFilter {
    fn name(&self) -> &str {
        "string.tolower"
    }

    fn filter_read(&self, data: &[u8]) -> Vec<u8> {
        data.iter().map(|b| b.to_ascii_lowercase()).collect()
    }

    fn filter_write(&self, data: &[u8]) -> Vec<u8> {
        data.iter().map(|b| b.to_ascii_lowercase()).collect()
    }
}

/// Built-in string.rot13 filter
pub struct StringRot13Filter;

impl StreamFilter for StringRot13Filter {
    fn name(&self) -> &str {
        "string.rot13"
    }

    fn filter_read(&self, data: &[u8]) -> Vec<u8> {
        rot13(data)
    }

    fn filter_write(&self, data: &[u8]) -> Vec<u8> {
        rot13(data)
    }
}

/// ROT13 transformation
fn rot13(data: &[u8]) -> Vec<u8> {
    data.iter()
        .map(|&b| match b {
            b'A'..=b'M' | b'a'..=b'm' => b + 13,
            b'N'..=b'Z' | b'n'..=b'z' => b - 13,
            _ => b,
        })
        .collect()
}

/// Built-in convert.base64-encode filter
pub struct Base64EncodeFilter;

impl StreamFilter for Base64EncodeFilter {
    fn name(&self) -> &str {
        "convert.base64-encode"
    }

    fn filter_read(&self, data: &[u8]) -> Vec<u8> {
        base64_encode(data).into_bytes()
    }

    fn filter_write(&self, data: &[u8]) -> Vec<u8> {
        base64_encode(data).into_bytes()
    }
}

/// Built-in convert.base64-decode filter
pub struct Base64DecodeFilter;

impl StreamFilter for Base64DecodeFilter {
    fn name(&self) -> &str {
        "convert.base64-decode"
    }

    fn filter_read(&self, data: &[u8]) -> Vec<u8> {
        base64_decode(data).unwrap_or_default()
    }

    fn filter_write(&self, data: &[u8]) -> Vec<u8> {
        base64_decode(data).unwrap_or_default()
    }
}

/// Simple base64 encoding (no external dependency)
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Simple base64 decoding
fn base64_decode(data: &[u8]) -> Result<Vec<u8>, String> {
    fn decode_char(c: u8) -> Result<u8, String> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            b'=' => Ok(0),
            _ => Err(format!("Invalid base64 character: {}", c as char)),
        }
    }

    let filtered: Vec<u8> = data
        .iter()
        .copied()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    let mut result = Vec::new();
    for chunk in filtered.chunks(4) {
        if chunk.len() < 4 {
            break;
        }
        let b0 = decode_char(chunk[0])?;
        let b1 = decode_char(chunk[1])?;
        let b2 = decode_char(chunk[2])?;
        let b3 = decode_char(chunk[3])?;
        let triple = ((b0 as u32) << 18) | ((b1 as u32) << 12) | ((b2 as u32) << 6) | (b3 as u32);
        result.push(((triple >> 16) & 0xFF) as u8);
        if chunk[2] != b'=' {
            result.push(((triple >> 8) & 0xFF) as u8);
        }
        if chunk[3] != b'=' {
            result.push((triple & 0xFF) as u8);
        }
    }
    Ok(result)
}

/// Filter chain - applies multiple filters in sequence
pub struct FilterChain {
    filters: Vec<Box<dyn StreamFilter>>,
}

impl FilterChain {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    /// Append a filter to the chain
    pub fn append(&mut self, filter: Box<dyn StreamFilter>) {
        self.filters.push(filter);
    }

    /// Prepend a filter to the chain
    pub fn prepend(&mut self, filter: Box<dyn StreamFilter>) {
        self.filters.insert(0, filter);
    }

    /// Remove a filter by name
    pub fn remove(&mut self, name: &str) -> bool {
        let len_before = self.filters.len();
        self.filters.retain(|f| f.name() != name);
        self.filters.len() < len_before
    }

    /// Apply all filters on read
    pub fn apply_read(&self, data: &[u8]) -> Vec<u8> {
        let mut result = data.to_vec();
        for filter in &self.filters {
            result = filter.filter_read(&result);
        }
        result
    }

    /// Apply all filters on write
    pub fn apply_write(&self, data: &[u8]) -> Vec<u8> {
        let mut result = data.to_vec();
        for filter in &self.filters {
            result = filter.filter_write(&result);
        }
        result
    }

    /// Get the number of filters in the chain
    pub fn len(&self) -> usize {
        self.filters.len()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }
}

impl Default for FilterChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a stream filter by name (PHP's stream_filter_append equivalent)
pub fn create_filter(name: &str) -> StreamResult<Box<dyn StreamFilter>> {
    match name {
        "string.toupper" => Ok(Box::new(StringToUpperFilter)),
        "string.tolower" => Ok(Box::new(StringToLowerFilter)),
        "string.rot13" => Ok(Box::new(StringRot13Filter)),
        "convert.base64-encode" => Ok(Box::new(Base64EncodeFilter)),
        "convert.base64-decode" => Ok(Box::new(Base64DecodeFilter)),
        _ => Err(StreamError::InvalidOperation),
    }
}
