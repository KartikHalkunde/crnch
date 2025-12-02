use colored::*;
use std::io::{self, Write};
use std::time::Instant;
use std::process::Command;
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};

// Verbosity levels: 0=quiet, 1=normal, 2=verbose, 3=nerd
static VERBOSITY: AtomicU8 = AtomicU8::new(1);

pub fn set_verbosity(level: u8) {
    VERBOSITY.store(level, Ordering::Relaxed);
}

pub fn get_verbosity() -> u8 {
    VERBOSITY.load(Ordering::Relaxed)
}

// Legacy compatibility
#[allow(dead_code)]
pub fn set_nerd_mode(enabled: bool) {
    set_verbosity(if enabled { 3 } else { 1 });
}

pub fn is_nerd_mode() -> bool {
    get_verbosity() >= 3
}

// ==================== PACMAN PROGRESS BAR ====================

pub struct PacmanProgress {
    total: u64,
    current: u64,
    width: usize,
    start_time: Instant,
    message: String,
}

impl PacmanProgress {
    pub fn new(total: u64, message: &str) -> Self {
        let bar = Self {
            total,
            current: 0,
            width: 30,
            start_time: Instant::now(),
            message: message.to_string(),
        };
        bar.render();
        bar
    }

    pub fn set(&mut self, current: u64) {
        self.current = current.min(self.total);
        self.render();
    }

    fn render(&self) {
        if is_nerd_mode() { return; } // No progress bar in nerd mode

        let progress = if self.total > 0 {
            self.current as f64 / self.total as f64
        } else {
            0.0
        };

        let pacman_pos = (progress * self.width as f64) as usize;

        // Build the bar: spaces behind pacman, C for pacman, dots ahead
        let behind = " ".repeat(pacman_pos);
        let pacman = "C";
        let ahead_count = self.width.saturating_sub(pacman_pos + 1);
        let ahead = ".".repeat(ahead_count);

        let percent = (progress * 100.0) as u64;

        // Use ANSI escape codes to clear the line properly
        print!("\r\x1B[2K");  // Clear entire line
        print!("\r   [{}{}{}] {}% {}   ", 
            behind, 
            pacman.yellow(), 
            ahead.dimmed(),
            percent,
            self.message
        );
        io::stdout().flush().unwrap();
    }

    pub fn finish(&self) {
        if is_nerd_mode() { return; }
        
        let elapsed = self.start_time.elapsed();
        // Clear the entire line with ANSI escape code
        print!("\r\x1B[2K");
        // Final state: pacman at the end, all dots eaten
        let behind = " ".repeat(self.width);
        println!("\r   [{}{}] 100% Done! ({:.1}s)", 
            behind, 
            "C".green(),
            elapsed.as_secs_f64()
        );
    }

    pub fn finish_with_message(&self, msg: &str) {
        if is_nerd_mode() { return; }
        
        // Clear the entire line with ANSI escape code
        print!("\r\x1B[2K");
        let behind = " ".repeat(self.width);
        println!("\r   [{}{}] {}", behind, "C".green(), msg);
    }
}

// ==================== DEFAULT MODE LOGGING ====================

pub fn log_start(filename: &str) {
    if is_nerd_mode() { return; }
    println!("\n{} Crnching '{}'...", ">>".cyan(), filename);
}

pub fn log_target(target: &str) {
    if is_nerd_mode() { return; }
    println!("   Target: {}", target.cyan());
}

pub fn log_done() {
    if is_nerd_mode() { return; }
    println!("{}", ">> Done!".green());
}

pub fn log_result(input_path: &str, output_path: &str, old_kb: u64, new_kb: u64) {
    if is_nerd_mode() { return; }
    
    log_summary(input_path, output_path, old_kb, new_kb, None, None);
}

/// Enhanced summary output with detailed compression statistics
pub fn log_summary(
    input_path: &str, 
    output_path: &str, 
    old_kb: u64, 
    new_kb: u64, 
    method: Option<&str>,
    time_ms: Option<u128>
) {
    if is_nerd_mode() { return; }
    
    let reduction_pct = if old_kb > 0 && new_kb <= old_kb {
        (old_kb - new_kb) as f64 / old_kb as f64 * 100.0
    } else { 0.0 };
    
    let saved_kb = old_kb.saturating_sub(new_kb);
    let ratio = if new_kb > 0 { old_kb as f64 / new_kb as f64 } else { 1.0 };
    
    // Format file sizes nicely
    let old_size_str = format_size(old_kb);
    let new_size_str = format_size(new_kb);
    
    println!();
    println!("{}", "┌─────────────────────────────────────────────────────────┐".dimmed());
    println!("{}", "│                    COMPRESSION SUMMARY                  │".cyan().bold());
    println!("{}", "├─────────────────────────────────────────────────────────┤".dimmed());
    
    // Input/Output files
    let in_name = Path::new(input_path).file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| input_path.to_string());
    let out_name = Path::new(output_path).file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| output_path.to_string());
    
    println!("  {} {}", "Input: ".dimmed(), in_name);
    println!("  {} {}", "Output:".dimmed(), out_name.green());
    
    println!("{}", "├─────────────────────────────────────────────────────────┤".dimmed());
    
    // Size info with visual bar
    let bar_width = 30;
    let (filled, bar_color) = if new_kb > old_kb {
        // File grew - show empty bar in red
        (0, "red")
    } else if old_kb > 0 {
        // Normal compression - green bar based on compression ratio
        let ratio = (new_kb as f64 / old_kb as f64 * bar_width as f64).round() as usize;
        (ratio.min(bar_width), "green")
    } else {
        (bar_width, "green")
    };
    let empty = bar_width - filled;
    
    let bar = if bar_color == "red" {
        format!("{}{}",
            "░".repeat(empty).red(),
            "█".repeat(filled).red()
        )
    } else {
        format!("{}{}",
            "█".repeat(filled).green(),
            "░".repeat(empty).dimmed()
        )
    };
    
    println!("  {} {} → {}", "Size:  ".dimmed(), old_size_str, new_size_str.green());
    println!("         [{}]", bar);
    
    // Statistics
    if new_kb > old_kb {
        let increase_msg = if old_kb == 0 {
            "file grew from < 1 KB".to_string()
        } else {
            let increase_pct = (new_kb - old_kb) as f64 / old_kb as f64 * 100.0;
            format!("file grew by {:.1}%", increase_pct)
        };
        println!("  {} {} ({})", 
            "Saved: ".dimmed(), 
            "0%".yellow(),
            increase_msg.yellow()
        );
    } else {
        println!("  {} {} ({} saved, {:.2}:1 ratio)", 
            "Saved: ".dimmed(),
            format!("{:.1}%", reduction_pct).green().bold(),
            format_size(saved_kb).green(),
            ratio
        );
    }
    
    // Optional method info (verbose mode)
    if let Some(m) = method {
        println!("  {} {}", "Method:".dimmed(), m.cyan());
    }
    
    // Optional timing info
    if let Some(ms) = time_ms {
        let time_str = if ms >= 1000 {
            format!("{:.2}s", ms as f64 / 1000.0)
        } else {
            format!("{}ms", ms)
        };
        println!("  {} {}", "Time:  ".dimmed(), time_str);
    }
    
    println!("{}", "└─────────────────────────────────────────────────────────┘".dimmed());
}

#[allow(dead_code)]
pub fn nerd_final_result(_dpi: u64, _old_kb: u64, _new_kb: u64, _iterations: usize, _time_ms: u128) {
    // Placeholder for potential future use
}

/// Format size in human-readable form
fn format_size(kb: u64) -> String {
    if kb >= 1024 {
        format!("{:.1} MB", kb as f64 / 1024.0)
    } else if kb == 0 {
        // File is less than 1KB, show as bytes
        "< 1 KB".to_string()
    } else {
        format!("{} KB", kb)
    }
}

pub fn log_warning(msg: &str) {
    println!("\n{} {}", "WARNING:".yellow().bold(), msg);
}

pub fn log_error(msg: &str) {
    println!("{} {}", "ERROR:".red().bold(), msg);
}

// ==================== NERD MODE LOGGING ====================

pub fn nerd_header() {
    if !is_nerd_mode() { return; }
    
    // Get system info
    let os_info = get_os_info();
    let arch = get_arch();
    let gs_version = get_tool_version("gs", &["--version"]);
    let magick_version = get_tool_version("magick", &["--version"]);
    let pngquant_version = get_tool_version("pngquant", &["--version"]);
    let cpu_info = get_cpu_info();
    let mem_info = get_mem_info();
    
    println!("\n{}", "╔═══════════════════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║                          SYSTEM INFORMATION                           ║".cyan().bold());
    println!("{}", "╠═══════════════════════════════════════════════════════════════════════╣".cyan());
    println!("  {} {:<25} {} {}", "OS:".dimmed(), os_info, "Arch:".dimmed(), arch);
    println!("  {} {}", "CPU:".dimmed(), cpu_info);
    println!("  {} {}", "RAM:".dimmed(), mem_info);
    println!("{}", "╠═══════════════════════════════════════════════════════════════════════╣".cyan());
    println!("  {} {:<40}", "Ghostscript:".green(), gs_version);
    println!("  {} {:<40}", "ImageMagick:".green(), magick_version);
    println!("  {} {:<40}", "pngquant:   ".green(), pngquant_version);
    println!("{}", "╚═══════════════════════════════════════════════════════════════════════╝".cyan());
}

pub fn nerd_file_info(input: &str, size_kb: u64, target_kb: Option<u64>) {
    if !is_nerd_mode() { return; }
    
    let path = Path::new(input);
    let filename = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().to_uppercase()).unwrap_or_default();
    let abs_path = std::fs::canonicalize(input).map(|p| p.display().to_string()).unwrap_or(input.to_string());
    
    println!("\n{}", "╔═══════════════════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║                            INPUT FILE                                 ║".cyan().bold());
    println!("{}", "╠═══════════════════════════════════════════════════════════════════════╣".cyan());
    println!("  {} {}", "Filename:".dimmed(), filename.green());
    println!("  {} {}", "Type:    ".dimmed(), ext.yellow());
    println!("  {} {}", "Path:    ".dimmed(), abs_path.dimmed());
    
    // Show actual file size in bytes if we have it
    if let Ok(metadata) = std::fs::metadata(input) {
        let bytes = metadata.len();
        if bytes < 1024 {
            println!("  {} {} bytes", "Size:    ".dimmed(), bytes);
        } else if bytes < 1024 * 1024 {
            println!("  {} {:.2} KB ({} bytes)", "Size:    ".dimmed(), bytes as f64 / 1024.0, bytes);
        } else {
            println!("  {} {:.2} MB ({} bytes)", "Size:    ".dimmed(), bytes as f64 / (1024.0 * 1024.0), bytes);
        }
    } else {
        println!("  {} {} KB (approx)", "Size:    ".dimmed(), size_kb);
    }
    
    // Try to get image dimensions for JPG/PNG
    if ext == "JPG" || ext == "JPEG" || ext == "PNG" {
        if let Some((width, height)) = get_image_dimensions(input) {
            println!("  {} {}x{} pixels", "Dimensions:".dimmed(), width, height);
            let megapixels = (width * height) as f64 / 1_000_000.0;
            println!("  {} {:.2} MP", "Resolution:".dimmed(), megapixels);
        }
    }
    
    println!("{}", "╠═══════════════════════════════════════════════════════════════════════╣".cyan());
    
    if let Some(target) = target_kb {
        let reduction = if size_kb > 0 && size_kb > target {
            ((size_kb - target) as f64 / size_kb as f64 * 100.0) as u64
        } else { 0 };
        let ratio_needed = if target > 0 { size_kb as f64 / target as f64 } else { 0.0 };
        println!("  {} {} KB", "Target:  ".dimmed(), target.to_string().cyan());
        println!("  {} {}%", "Reduction:".dimmed(), reduction.to_string().yellow());
        println!("  {} {:.2}:1", "Ratio:   ".dimmed(), ratio_needed.to_string().green());
    } else {
        println!("  {} Auto (preset-based)", "Target:  ".dimmed());
    }
    println!("{}", "╚═══════════════════════════════════════════════════════════════════════╝".cyan());
}

pub fn nerd_stage(stage_num: u32, name: &str) {
    if !is_nerd_mode() { return; }
    println!("\n{}", "─".repeat(75).dimmed());
    println!("{} {}", format!("[STAGE {}]", stage_num).yellow().bold(), name.bold());
    println!("{}", "─".repeat(75).dimmed());
}

pub fn nerd_cmd(cmd_str: &str) {
    if !is_nerd_mode() { return; }
    println!("  ├─ Cmd: {}", cmd_str.dimmed());
}

pub fn nerd_attempt(attempt: u32, max: u32, dpi: u64, size_kb: u64, target_kb: u64, time_ms: u128, action: &str) {
    if !is_nerd_mode() { return; }
    
    let delta = if size_kb > target_kb {
        format!("+{} KB", size_kb - target_kb).red()
    } else {
        format!("-{} KB", target_kb - size_kb).green()
    };
    
    let status_icon = if size_kb <= target_kb { "OK".green() } else { "XX".red() };
    
    let prefix = if attempt == max { "  └─" } else { "  ├─" };
    println!("{} [{:>2}/{}] {:>4} DPI -> {:>4} KB [{}] ({}) | {}ms | next: {}", 
        prefix, attempt, max, dpi, size_kb, status_icon, delta, time_ms, action.dimmed());
}

pub fn nerd_quality_attempt(attempt: u32, max: u32, quality: u8, size_kb: u64, target_kb: u64, time_ms: u128, action: &str) {
    if !is_nerd_mode() { return; }
    
    let delta = if size_kb > target_kb {
        format!("+{} KB", size_kb - target_kb).red()
    } else {
        format!("-{} KB", target_kb - size_kb).green()
    };
    
    let status_icon = if size_kb <= target_kb { "OK".green() } else { "XX".red() };
    
    let prefix = if attempt == max { "  └─" } else { "  ├─" };
    println!("{} [{:>2}] Quality {:>3}% -> {:>4} KB [{}] ({}) | {}ms | next: {}", 
        prefix, attempt, quality, size_kb, status_icon, delta, time_ms, action.dimmed());
}

pub fn nerd_scale_attempt(attempt: u32, max: u32, scale: u8, size_kb: u64, target_kb: u64, time_ms: u128, action: &str) {
    if !is_nerd_mode() { return; }
    
    let delta = if size_kb > target_kb {
        format!("+{} KB", size_kb - target_kb).red()
    } else {
        format!("-{} KB", target_kb - size_kb).green()
    };
    
    let status_icon = if size_kb <= target_kb { "OK".green() } else { "XX".red() };
    
    let prefix = if attempt == max { "  └─" } else { "  ├─" };
    println!("{} [{:>2}] Scale {:>3}% -> {:>4} KB [{}] ({}) | {}ms | next: {}", 
        prefix, attempt, scale, size_kb, status_icon, delta, time_ms, action.dimmed());
}

pub fn nerd_result(label: &str, value: &str, is_last: bool) {
    if !is_nerd_mode() { return; }
    let prefix = if is_last { "  └─" } else { "  ├─" };
    if value.is_empty() {
        println!("{} {}", prefix.dimmed(), label.yellow());
    } else {
        println!("{} {} {}", prefix.dimmed(), format!("{}:", label).dimmed(), value);
    }
}

pub fn nerd_output_summary(_input: &str, output: &str, old_kb: u64, new_kb: u64, method: &str, time_s: f64) {
    if !is_nerd_mode() { return; }
    
    let reduction_pct = if old_kb > 0 && new_kb <= old_kb {
        (old_kb - new_kb) as f64 / old_kb as f64 * 100.0
    } else { 0.0 };
    
    let ratio = if new_kb > 0 { old_kb as f64 / new_kb as f64 } else { 1.0 };
    let saved_kb = old_kb.saturating_sub(new_kb);
    
    println!("\n{}", "╔═══════════════════════════════════════════════════════════════════════╗".green());
    println!("{}", "║                         COMPRESSION RESULT                            ║".green().bold());
    println!("{}", "╠═══════════════════════════════════════════════════════════════════════╣".green());
    
    let out_name = Path::new(output).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| output.to_string());
    println!("  {} {}", "Output File:".dimmed(), out_name.green());
    println!("  {} {}", "Method:     ".dimmed(), method.cyan());
    println!("{}", "╠═══════════════════════════════════════════════════════════════════════╣".green());
    
    let old_size_str = if old_kb >= 1024 {
        format!("{:.2} MB", old_kb as f64 / 1024.0)
    } else if old_kb == 0 {
        "< 1 KB".to_string()
    } else {
        format!("{} KB", old_kb)
    };
    
    let new_size_str = if new_kb >= 1024 {
        format!("{:.2} MB", new_kb as f64 / 1024.0)
    } else if new_kb == 0 {
        "< 1 KB".to_string()
    } else {
        format!("{} KB", new_kb)
    };
    
    println!("  {} {} → {}", "Size:       ".dimmed(), old_size_str, new_size_str.green());
    println!("  {} {:.1}% ({} KB saved)", "Reduction:  ".dimmed(), reduction_pct, saved_kb);
    println!("  {} {:.2}:1", "Ratio:      ".dimmed(), ratio);
    println!("  {} {:.2}s", "Time:       ".dimmed(), time_s);
    
    println!("{}", "╚═══════════════════════════════════════════════════════════════════════╝".green());
}

// Binary search visualization helper
pub fn nerd_search_range(min: u64, max: u64, mid: u64) {
    if !is_nerd_mode() { return; }
    
    // Visual representation of search range
    let total_range = 2400u64;
    let width = 50usize;
    
    let min_pos = (min as f64 / total_range as f64 * width as f64) as usize;
    let max_pos = (max as f64 / total_range as f64 * width as f64) as usize;
    let mid_pos = (mid as f64 / total_range as f64 * width as f64) as usize;
    
    let mut bar = vec!['.'; width];
    for item in bar.iter_mut().take(max_pos.min(width - 1) + 1).skip(min_pos) {
        *item = '=';
    }
    if mid_pos < width {
        bar[mid_pos] = '^';
    }
    
    let bar_str: String = bar.iter().collect();
    println!("  ├─ Range: [{}]", bar_str.dimmed());
    println!("  ├─         {} DPI{}{} DPI", 
        min, 
        " ".repeat(mid_pos.saturating_sub(min_pos.to_string().len())),
        max
    );
}

// ==================== HELPERS ====================

fn get_os_info() -> String {
    #[cfg(target_os = "linux")]
    {
        // Try to get distro info from /etc/os-release (works on most Linux distros)
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            let pretty_name = content.lines()
                .find(|line| line.starts_with("PRETTY_NAME="))
                .and_then(|line| line.split('=').nth(1))
                .map(|s| s.trim_matches('"').to_string());
            
            if let Some(name) = pretty_name {
                return name;
            }
        }
        
        // Fallback to kernel version
        Command::new("uname")
            .arg("-sr")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "Linux".to_string())
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .map(|o| format!("macOS {}", String::from_utf8_lossy(&o.stdout).trim()))
            .unwrap_or_else(|_| "macOS".to_string())
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "ver"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "Windows".to_string())
    }
    
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        "Unknown OS".to_string()
    }
}

fn get_arch() -> String {
    #[cfg(target_os = "windows")]
    {
        std::env::var("PROCESSOR_ARCHITECTURE").unwrap_or_else(|_| "Unknown".to_string())
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("uname")
            .arg("-m")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "Unknown".to_string())
    }
}

fn get_cpu_info() -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|content| {
                content.lines()
                    .find(|line| line.starts_with("model name"))
                    .and_then(|line| line.split(':').nth(1))
                    .map(|s| s.trim().to_string())
            })
            .unwrap_or_else(|| "Unknown".to_string())
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("sysctl")
            .arg("-n")
            .arg("machdep.cpu.brand_string")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "Unknown".to_string())
    }
    
    #[cfg(target_os = "windows")]
    {
        std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "Unknown".to_string())
    }
    
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        "Unknown".to_string()
    }
}

fn get_mem_info() -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|content| {
                content.lines()
                    .find(|line| line.starts_with("MemTotal"))
                    .and_then(|line| line.split_whitespace().nth(1))
                    .and_then(|kb| kb.parse::<u64>().ok())
                    .map(|kb| format!("{:.1} GB", kb as f64 / 1024.0 / 1024.0))
            })
            .unwrap_or_else(|| "Unknown".to_string())
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("sysctl")
            .arg("-n")
            .arg("hw.memsize")
            .output()
            .ok()
            .and_then(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse::<u64>()
                    .ok()
                    .map(|bytes| format!("{:.1} GB", bytes as f64 / 1024.0 / 1024.0 / 1024.0))
            })
            .unwrap_or_else(|| "Unknown".to_string())
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("wmic")
            .args(["ComputerSystem", "get", "TotalPhysicalMemory"])
            .output()
            .ok()
            .and_then(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .nth(1)
                    .and_then(|line| line.trim().parse::<u64>().ok())
                    .map(|bytes| format!("{:.1} GB", bytes as f64 / 1024.0 / 1024.0 / 1024.0))
            })
            .unwrap_or_else(|| "Unknown".to_string())
    }
    
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        "Unknown".to_string()
    }
}

fn get_tool_version(tool: &str, args: &[&str]) -> String {
    Command::new(tool)
        .args(args)
        .output()
        .map(|o| {
            let out = String::from_utf8_lossy(&o.stdout);
            out.lines().next().unwrap_or("Unknown").trim().to_string()
        })
        .unwrap_or_else(|_| "Not found".red().to_string())
}

fn get_image_dimensions(path: &str) -> Option<(u32, u32)> {
    // Try using ImageMagick's identify command
    Command::new("magick")
        .args(["identify", "-format", "%w %h", path])
        .output()
        .ok()
        .and_then(|output| {
            let s = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = s.split_whitespace().collect();
            if parts.len() >= 2 {
                let width = parts[0].parse::<u32>().ok()?;
                let height = parts[1].parse::<u32>().ok()?;
                Some((width, height))
            } else {
                None
            }
        })
}
