//! IRQ Smart PDF Engine - File inspection and verification.
//!
//! Provides dependency-free verification of downloaded PDF documents,
//! ensuring that HTML login pages, paywalls, or server error pages
//! are rejected with clear diagnostics.

use anyhow::{bail, Result};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Check if a byte slice contains the standard PDF magic signature `%PDF-`
/// within the first 1024 bytes.
pub fn is_pdf_signature(bytes: &[u8]) -> bool {
    let limit = bytes.len().min(1024);
    bytes[..limit].windows(5).any(|w| w == b"%PDF-")
}

/// Check the PDF signature using a reader (PDF readers allow up to 1024 bytes of preamble).
pub fn has_signature(reader: &mut (impl Read + Seek)) -> std::io::Result<bool> {
    let mut header = [0u8; 1024];
    reader.seek(SeekFrom::Start(0))?;
    let mut read = 0;
    while read < header.len() {
        let n = reader.read(&mut header[read..])?;
        if n == 0 {
            break;
        }
        read += n;
    }
    Ok(is_pdf_signature(&header[..read]))
}

/// Verify bytes directly, diagnosing specific reasons for rejection.
pub fn verify_bytes(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() {
        bail!("Downloaded file is empty (0 bytes)");
    }
    if is_pdf_signature(bytes) {
        return Ok(());
    }

    let limit = bytes.len().min(1024);
    let sample = &bytes[..limit];
    let sample_lower = sample.to_ascii_lowercase();

    if sample_lower.windows(5).any(|w| w == b"<!doc" || w == b"<html")
        || sample_lower.windows(6).any(|w| w == b"<head>" || w == b"<body>")
        || sample_lower.windows(7).any(|w| w == b"<script")
    {
        bail!("Downloaded file is an HTML page (such as a login, paywall, or captcha page) instead of a PDF document");
    }

    if sample_lower.windows(7).any(|w| w == b"{\"error" || w == b"\"error\"") {
        bail!("Downloaded file is a JSON error response instead of a PDF document");
    }

    if sample_lower.windows(3).any(|w| w == b"403" || w == b"404" || w == b"500") {
        bail!("Downloaded file contains a server error response instead of a PDF document");
    }

    bail!("Downloaded file is not a valid PDF: missing %PDF- signature (the server may have returned a login or error page)");
}

/// Fail a completed `.pdf` download if the server returned HTML, an error page, or corrupt data.
pub fn verify_file(path: &Path) -> Result<()> {
    let mut file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    if metadata.len() == 0 {
        bail!("Downloaded file is empty (0 bytes)");
    }
    let mut buffer = [0u8; 1024];
    let n = file.read(&mut buffer)?;
    verify_bytes(&buffer[..n])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn accepts_pdf_signature_near_the_start() {
        let mut bytes = vec![b' '; 500];
        bytes.extend_from_slice(b"%PDF-1.7\n");
        assert!(has_signature(&mut Cursor::new(&bytes)).unwrap());
        assert!(verify_bytes(&bytes).is_ok());
    }

    #[test]
    fn rejects_empty_data() {
        assert!(verify_bytes(&[]).is_err());
    }

    #[test]
    fn rejects_html_login_and_error_pages() {
        for body in [
            b"<!doctype html><title>Sign in</title>".as_slice(),
            b"<html><body><h1>Access Denied</h1></body></html>".as_slice(),
            b"403 Forbidden - Access to resource denied".as_slice(),
            b"{\"error\": \"Unauthorized\", \"code\": 401}".as_slice(),
        ] {
            assert!(!has_signature(&mut Cursor::new(body)).unwrap());
            assert!(verify_bytes(body).is_err());
        }
    }
}
