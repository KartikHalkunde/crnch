use colored::*;
use std::io::{self, Write};
use std::time::{Duration, Instant};
use std::process::Command;
use std::path::Path;

// Global nerd mode flag (set once at startup)
static mut NERD_MODE: bool = false;

pub fn set_nerd_mode(enabled: bool) {
    unsafe { NERD_MODE = enabled; }
}

pub fn is_nerd_mode() -> bool {
    unsafe { NERD_MODE }
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
        // Final state: pacman at the end, all dots eaten
        let behind = " ".repeat(self.width);
        // Pad with extra spaces to clear any leftover text
        println!("\r   [{}{}] 100% Done! ({:.1}s)   {}", 
            behind, 
            "C".green(),
            elapsed.as_secs_f64(),
            "      " // extra spaces
        );
    }

    pub fn finish_with_message(&self, msg: &str) {
        if is_nerd_mode() { return; }
        
        let behind = " ".repeat(self.width);
        println!("\r   [{}{}] {}   ", behind, "C".green(), msg);
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
    
    let reduction = if old_kb > 0 {
        ((old_kb - new_kb) as f64 / old_kb as f64 * 100.0) as u64
    } else { 0 };
    
    println!("   Input:  {} ({}KB)", input_path, old_kb);
    println!("   Output: {} ({}KB)", output_path, new_kb.to_string().green());
    println!("   Saved:  {}%", reduction.to_string().green());
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
    
    println!("\n{}", "=".repeat(70).dimmed());
    println!("{}", "[SYSTEM INFO]".cyan().bold());
    println!("   |- OS: {}", os_info);
    println!("   |- Arch: {}", arch);
    println!("   |- CPU: {}", cpu_info);
    println!("   |- RAM: {}", mem_info);
    println!("   |- Ghostscript: {}", gs_version);
    println!("   |- ImageMagick: {}", magick_version);
    println!("   '- pngquant: {}", pngquant_version);
    println!("{}", "=".repeat(70).dimmed());
}

pub fn nerd_file_info(input: &str, size_kb: u64, target_kb: Option<u64>) {
    if !is_nerd_mode() { return; }
    
    let path = Path::new(input);
    let filename = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().to_uppercase()).unwrap_or_default();
    let abs_path = std::fs::canonicalize(input).map(|p| p.display().to_string()).unwrap_or(input.to_string());
    
    println!("\n{}", "[INPUT FILE]".cyan().bold());
    println!("   |- File: {}", filename);
    println!("   |- Type: {}", ext);
    println!("   |- Path: {}", abs_path.dimmed());
    println!("   |- Size: {} KB ({} bytes)", size_kb, size_kb * 1024);
    
    if let Some(target) = target_kb {
        let reduction = if size_kb > 0 && size_kb > target {
            ((size_kb - target) as f64 / size_kb as f64 * 100.0) as u64
        } else { 0 };
        let ratio_needed = if target > 0 { size_kb as f64 / target as f64 } else { 0.0 };
        println!("   |- Target: {} KB", target);
        println!("   |- Reduction Needed: {}%", reduction);
        println!("   '- Compression Ratio Needed: {:.2}:1", ratio_needed);
    } else {
        println!("   '- Target: Auto (preset-based)");
    }
}

pub fn nerd_stage(stage_num: u32, name: &str) {
    if !is_nerd_mode() { return; }
    println!("\n{}", format!("[STAGE {}] {}", stage_num, name).yellow().bold());
}

pub fn nerd_algo(name: &str, complexity: &str, details: &str) {
    if !is_nerd_mode() { return; }
    println!("   |- Algorithm: {}", name.green());
    println!("   |- Complexity: {}", complexity.cyan());
    if !details.is_empty() {
        println!("   |- Strategy: {}", details);
    }
}

pub fn nerd_cmd(cmd_str: &str) {
    if !is_nerd_mode() { return; }
    println!("   |- Cmd: {}", cmd_str.dimmed());
}

pub fn nerd_attempt(attempt: u32, max: u32, dpi: u64, size_kb: u64, target_kb: u64, time_ms: u128, action: &str) {
    if !is_nerd_mode() { return; }
    
    let delta = if size_kb > target_kb {
        format!("+{} KB", size_kb - target_kb).red()
    } else {
        format!("-{} KB", target_kb - size_kb).green()
    };
    
    let status_icon = if size_kb <= target_kb { "OK".green() } else { "XX".red() };
    
    let prefix = if attempt == max { "   '-" } else { "   |-" };
    println!("{} [{:>2}/{}] {:>4} DPI -> {:>4} KB [{}] ({}) | {}ms | next: {}", 
        prefix, attempt, max, dpi, size_kb, status_icon, delta, time_ms, action.dimmed());
}

pub fn nerd_result(label: &str, value: &str, is_last: bool) {
    if !is_nerd_mode() { return; }
    let prefix = if is_last { "   '-" } else { "   |-" };
    println!("{} {}: {}", prefix, label, value);
}

pub fn nerd_final_result(dpi: u64, old_kb: u64, new_kb: u64, iterations: u32, max_iterations: u32, total_time: Duration, gs_calls: u32, output: &str) {
    if !is_nerd_mode() { return; }
    
    let reduction = if old_kb > 0 {
        (old_kb - new_kb) as f64 / old_kb as f64 * 100.0
    } else { 0.0 };
    
    let ratio = if new_kb > 0 {
        old_kb as f64 / new_kb as f64
    } else { 0.0 };
    
    let efficiency = if max_iterations > 0 {
        (max_iterations - iterations) as f64 / max_iterations as f64 * 100.0
    } else { 0.0 };
    
    let avg_time_per_call = if gs_calls > 0 {
        total_time.as_millis() as f64 / gs_calls as f64
    } else { 0.0 };
    
    // Quality assessment based on DPI
    let quality = if dpi >= 300 {
        "Excellent (print-ready)".green()
    } else if dpi >= 150 {
        "Good (screen quality)".green()
    } else if dpi >= 72 {
        "Fair (web quality)".yellow()
    } else {
        "Low (may be pixelated)".red()
    };
    
    println!("\n{}", "=".repeat(70).dimmed());
    println!("{}", "[COMPRESSION RESULT]".green().bold());
    println!("   |- Optimal DPI: {}", dpi.to_string().cyan());
    println!("   |- Quality: {}", quality);
    println!("   |- Size: {} KB -> {} KB", old_kb, new_kb.to_string().green());
    println!("   |- Reduction: {:.1}%", reduction);
    println!("   |- Compression Ratio: {:.2}:1", ratio);
    println!("   |");
    println!("   |- {} {}", "[PERFORMANCE]".cyan(), "".dimmed());
    println!("   |- Iterations: {}/{} ({:.0}% early-exit)", iterations, max_iterations, efficiency);
    println!("   |- GS Calls: {}", gs_calls);
    println!("   |- Total Time: {:.2}s", total_time.as_secs_f64());
    println!("   |- Avg per GS call: {:.0}ms", avg_time_per_call);
    println!("   |");
    println!("   '- Output: {}", output.green());
    println!("{}", "=".repeat(70).dimmed());
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
    for i in min_pos..=max_pos.min(width - 1) {
        bar[i] = '=';
    }
    if mid_pos < width {
        bar[mid_pos] = '^';
    }
    
    let bar_str: String = bar.iter().collect();
    println!("   |- Range: [{}]", bar_str.dimmed());
    println!("   |-         {} DPI{}{} DPI", 
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
