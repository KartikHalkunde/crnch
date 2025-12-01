use regex::Regex;

/// Parse a size string like "200k", "1.5m", "500kb", "2mb" into KB
pub fn parse_size(size_str: &str) -> Option<u64> {
    let re = Regex::new(r"(?i)^(\d+(?:\.\d+)?)(k|m|kb|mb)?$").ok()?;
    let caps = re.captures(size_str)?;
    let val: f64 = caps[1].parse().ok()?;
    let unit = caps.get(2).map_or("k", |m| m.as_str()).to_lowercase();
    match unit.as_str() {
        "m" | "mb" => Some((val * 1024.0) as u64),
        _ => Some(val as u64),
    }
}
