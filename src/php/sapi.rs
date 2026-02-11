//! PHP SAPI
//!
//! Server API layer
//! Migrated from main/SAPI.h and main/SAPI.c

#[cfg(test)]
mod tests;

/// SAPI header structure
#[derive(Debug, Clone)]
pub struct SapiHeader {
    pub header: String,
    pub header_len: usize,
}

/// SAPI headers collection
#[derive(Debug, Default)]
pub struct SapiHeaders {
    pub headers: Vec<SapiHeader>,
    pub http_response_code: i32,
    pub send_default_content_type: bool,
    pub mimetype: Option<String>,
    pub http_status_line: Option<String>,
}

/// Request information
#[derive(Debug, Default)]
pub struct SapiRequestInfo {
    pub request_method: Option<String>,
    pub query_string: Option<String>,
    pub cookie_data: Option<String>,
    pub content_length: i64,
    pub path_translated: Option<String>,
    pub request_uri: Option<String>,
    pub content_type: Option<String>,
    pub headers_only: bool,
    pub no_headers: bool,
    pub headers_read: bool,
    pub argc: i32,
    pub argv: Vec<String>,
}

/// SAPI module structure
pub struct SapiModule {
    pub name: String,
    pub pretty_name: String,
    pub version: String,
    pub request_info: SapiRequestInfo,
    pub headers: SapiHeaders,
    pub started: bool,
}

impl SapiModule {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            pretty_name: name.to_string(),
            version: "0.1.0".to_string(),
            request_info: SapiRequestInfo::default(),
            headers: SapiHeaders::default(),
            started: false,
        }
    }

    /// Start the SAPI module
    pub fn startup(&mut self) -> Result<(), String> {
        self.started = true;
        Ok(())
    }

    /// Shutdown the SAPI module
    pub fn shutdown(&mut self) -> Result<(), String> {
        self.started = false;
        Ok(())
    }

    /// Activate the SAPI module
    pub fn activate(&mut self) -> Result<(), String> {
        if !self.started {
            return Err("SAPI module not started".to_string());
        }
        Ok(())
    }

    /// Deactivate the SAPI module
    pub fn deactivate(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Add a header
    pub fn add_header(&mut self, header: &str) {
        self.headers.headers.push(SapiHeader {
            header: header.to_string(),
            header_len: header.len(),
        });
    }

    /// Set HTTP response code
    pub fn set_response_code(&mut self, code: i32) {
        self.headers.http_response_code = code;
    }

    /// Set content type
    pub fn set_content_type(&mut self, mimetype: &str) {
        self.headers.mimetype = Some(mimetype.to_string());
    }
}

/// Global SAPI module instance (thread-safe)
static SAPI_MODULE: std::sync::Mutex<Option<SapiModule>> = std::sync::Mutex::new(None);

/// Initialize SAPI
pub fn sapi_startup(module: SapiModule) -> Result<(), String> {
    let mut guard = SAPI_MODULE.lock().map_err(|e| e.to_string())?;
    *guard = Some(module);
    if let Some(ref mut m) = *guard {
        m.startup()?;
    }
    Ok(())
}

/// Shutdown SAPI
pub fn sapi_shutdown() -> Result<(), String> {
    let mut guard = SAPI_MODULE.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut m) = *guard {
        m.shutdown()?;
    }
    *guard = None;
    Ok(())
}

/// Access the SAPI module via a closure
pub fn with_sapi_module<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&mut SapiModule) -> R,
{
    let mut guard = SAPI_MODULE.lock().map_err(|e| e.to_string())?;
    match guard.as_mut() {
        Some(m) => Ok(f(m)),
        None => Err("SAPI module not initialized".to_string()),
    }
}

/// CLI SAPI startup
pub fn php_cli_startup() -> Result<(), String> {
    let mut cli_module = SapiModule::new("cli");
    cli_module.pretty_name = "Command Line Interface".to_string();
    sapi_startup(cli_module)
}

/// CLI SAPI shutdown
pub fn php_cli_shutdown() -> Result<(), String> {
    sapi_shutdown()
}
