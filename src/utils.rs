use regex::Regex;
use anyhow::{Result, anyhow};

/// Parse a size string like "200k", "1.5m", "500kb", "2mb" into KB
pub fn parse_size(size_str: &str) -> Option<u64> {
    let re = Regex::new(r"(?i)^(\d+(?:\.\d+)?)(k|m|kb|mb|g|gb)?$").ok()?;
    let caps = re.captures(size_str)?;
    let val: f64 = caps[1].parse().ok()?;
    let unit = caps.get(2).map_or("k", |m| m.as_str()).to_lowercase();
    match unit.as_str() {
        "g" | "gb" => Some((val * 1024.0 * 1024.0) as u64),
        "m" | "mb" => Some((val * 1024.0) as u64),
        _ => Some(val as u64),
    }
}

/// Validate size string and provide helpful error message
pub fn validate_size(size_str: &str) -> Result<u64> {
    if size_str.is_empty() {
        return Err(anyhow!("Size cannot be empty. Examples: 200k, 1.5m, 500kb"));
    }
    
    match parse_size(size_str) {
        Some(0) => {
            Err(anyhow!("Size must be greater than 0. Examples: 200k, 1.5m, 500kb"))
        },
        Some(kb) if kb > 10485760 => { // 10GB limit
            Err(anyhow!("Size too large (max 10GB). Got: {}", size_str))
        },
        Some(kb) => Ok(kb),
        None => {
            Err(anyhow!(
                "Invalid size format: '{}'. Examples:\n   - 200k or 200kb (200 kilobytes)\n   - 1.5m or 1.5mb (1.5 megabytes)\n   - 2g or 2gb (2 gigabytes)",
                size_str
            ))
        }
    }
}

/// Validate file extension is supported
pub fn validate_file_extension(filename: &str) -> Result<String> {
    let path = std::path::Path::new(filename);
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .ok_or_else(|| anyhow!("File '{}' has no extension.\nSupported formats: .jpg, .jpeg, .png, .pdf", filename))?;
    
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "pdf" => Ok(ext),
        _ => Err(anyhow!(
            "Unsupported file type: .{}\nSupported formats: .jpg, .jpeg, .png, .pdf",
            ext
        ))
    }
}

/// Validate output path is writable
pub fn validate_output_path(output: &str) -> Result<()> {
    let path = std::path::Path::new(output);
    
    // Check for system directories
    let forbidden_paths = ["/etc", "/sys", "/proc", "/dev", "/boot", "/root"];
    for forbidden in &forbidden_paths {
        if output.starts_with(forbidden) {
            return Err(anyhow!("Cannot write to system directory: {}", forbidden));
        }
    }
    
    // Check parent directory exists and is writable
    if let Some(parent) = path.parent() {
        if parent.as_os_str().is_empty() {
            return Ok(()); // Current directory, assume writable
        }
        
        if !parent.exists() {
            return Err(anyhow!(
                "Output directory does not exist: {}\nCreate it first with: mkdir -p {}",
                parent.display(),
                parent.display()
            ));
        }
        
        // Check write permission
        let metadata = std::fs::metadata(parent)
            .map_err(|e| anyhow!("Cannot access directory {}: {}", parent.display(), e))?;
        
        if metadata.permissions().readonly() {
            return Err(anyhow!("Output directory is read-only: {}", parent.display()));
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size_kilobytes() {
        assert_eq!(parse_size("200k"), Some(200));
        assert_eq!(parse_size("200kb"), Some(200));
        assert_eq!(parse_size("200K"), Some(200));
        assert_eq!(parse_size("200KB"), Some(200));
    }

    #[test]
    fn test_parse_size_megabytes() {
        assert_eq!(parse_size("1m"), Some(1024));
        assert_eq!(parse_size("1mb"), Some(1024));
        assert_eq!(parse_size("1.5m"), Some(1536));
        assert_eq!(parse_size("2M"), Some(2048));
    }

    #[test]
    fn test_parse_size_gigabytes() {
        assert_eq!(parse_size("1g"), Some(1024 * 1024));
        assert_eq!(parse_size("1gb"), Some(1024 * 1024));
        assert_eq!(parse_size("2G"), Some(2 * 1024 * 1024));
    }

    #[test]
    fn test_parse_size_decimals() {
        assert_eq!(parse_size("0.5m"), Some(512));
        assert_eq!(parse_size("1.5k"), Some(1));
    }

    #[test]
    fn test_parse_size_invalid() {
        assert_eq!(parse_size(""), None);
        assert_eq!(parse_size("invalid"), None);
        assert_eq!(parse_size("k"), None);
        assert_eq!(parse_size("-100k"), None);
        assert_eq!(parse_size("100x"), None);
    }

    #[test]
    fn test_validate_size_success() {
        assert!(validate_size("200k").is_ok());
        assert!(validate_size("1.5m").is_ok());
        assert!(validate_size("1g").is_ok());
    }

    #[test]
    fn test_validate_size_zero() {
        assert!(validate_size("0k").is_err());
        assert!(validate_size("0").is_err());
    }

    #[test]
    fn test_validate_size_too_large() {
        assert!(validate_size("20g").is_err()); // > 10GB
    }

    #[test]
    fn test_validate_size_invalid_format() {
        assert!(validate_size("invalid").is_err());
        assert!(validate_size("").is_err());
        assert!(validate_size("-100k").is_err());
    }

    #[test]
    fn test_validate_file_extension_supported() {
        assert!(validate_file_extension("image.png").is_ok());
        assert!(validate_file_extension("photo.jpg").is_ok());
        assert!(validate_file_extension("photo.JPEG").is_ok());
        assert!(validate_file_extension("document.pdf").is_ok());
    }

    #[test]
    fn test_validate_file_extension_unsupported() {
        assert!(validate_file_extension("file.txt").is_err());
        assert!(validate_file_extension("file.zip").is_err());
        assert!(validate_file_extension("file.md").is_err());
    }

    #[test]
    fn test_validate_file_extension_no_extension() {
        assert!(validate_file_extension("file").is_err());
    }
}
