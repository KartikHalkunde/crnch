use std::process::Command;
use std::path::Path;
use anyhow::{Result, anyhow};
use regex::Regex;
use clap::ValueEnum;
use std::fs;
use std::time::Instant;
use dialoguer::Confirm;
use colored::*;
use crate::logger::{self, PacmanProgress};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum CompressionLevel {
    Low,    // Better Quality
    Medium, // Balanced
    High,   // Smallest size
}

fn parse_size(size_str: &str) -> Option<u64> {
    let re = Regex::new(r"(?i)^(\d+(?:\.\d+)?)(k|m|kb|mb)?$").ok()?;
    let caps = re.captures(size_str)?;
    let val: f64 = caps[1].parse().ok()?;
    let unit = caps.get(2).map_or("k", |m| m.as_str()).to_lowercase();

    match unit.as_str() {
        "m" | "mb" => Some((val * 1024.0) as u64),
        _ => Some(val as u64),
    }
}

fn get_file_size_kb(path: &str) -> u64 {
    fs::metadata(path).map(|m| m.len() / 1024).unwrap_or(0)
}

pub fn compress_file(input: &str, output: &str, size_str: Option<String>, level: Option<CompressionLevel>, nerd: bool) -> Result<()> {
    let path = Path::new(input);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let target_kb = if let Some(s) = size_str { parse_size(&s) } else { None };

    match ext.as_str() {
        "jpg" | "jpeg" => compress_jpg(input, output, target_kb, level, nerd),
        "png" => compress_png(input, output, target_kb, level, nerd),
        "pdf" => compress_pdf(input, output, target_kb, level, nerd),
        _ => Err(anyhow!("Unsupported file type: .{}", ext)),
    }
}

// ---------------------- ENGINES ----------------------

fn compress_jpg(input: &str, output: &str, target_kb: Option<u64>, level: Option<CompressionLevel>, nerd: bool) -> Result<()> {
    let start = Instant::now();
    
    if nerd {
        logger::nerd_stage(1, "JPEG Compression");
        logger::nerd_algo("ImageMagick extent", "O(1)", "Single-pass quality targeting");
    }
    
    let mut progress = PacmanProgress::new(2, "Compressing...");
    
    let mut cmd = Command::new("magick");
    cmd.arg(input).arg("-strip");

    if let Some(kb) = target_kb {
        cmd.arg("-define").arg(format!("jpeg:extent={}KB", kb));
        if nerd {
            logger::nerd_cmd(&format!("magick {} -strip -define jpeg:extent={}KB {}", input, kb, output));
        }
    } else if let Some(lvl) = level {
        let q = match lvl {
            CompressionLevel::Low => "85",
            CompressionLevel::Medium => "75",
            CompressionLevel::High => "50",
        };
        cmd.arg("-quality").arg(q);
        if nerd {
            logger::nerd_cmd(&format!("magick {} -strip -quality {} {}", input, q, output));
        }
    } else {
        cmd.arg("-quality").arg("80");
    }

    progress.set(1);
    cmd.arg(output);
    let status = cmd.status()?;
    progress.set(2);
    progress.finish();
    
    if nerd {
        let elapsed = start.elapsed();
        logger::nerd_result("Time", &format!("{:.0}ms", elapsed.as_millis()), false);
        logger::nerd_result("Status", if status.success() { "Success" } else { "Failed" }, true);
    }
    
    if !status.success() { return Err(anyhow!("ImageMagick failed.")); }
    Ok(())
}

fn compress_png(input: &str, output: &str, target_kb: Option<u64>, level: Option<CompressionLevel>, nerd: bool) -> Result<()> {
    let start = Instant::now();
    
    if nerd {
        logger::nerd_stage(1, "PNG Color Quantization");
        logger::nerd_algo("pngquant", "O(n)", "Iterative quality reduction");
    }
    
    // 1. Color Compression Loop
    let attempts = if let Some(_) = target_kb {
        vec!["60-80", "30-60", "10-30"] 
    } else if let Some(lvl) = level {
        match lvl {
            CompressionLevel::Low => vec!["80-95"],
            CompressionLevel::Medium => vec!["60-80"],
            CompressionLevel::High => vec!["30-50"],
        }
    } else {
        vec!["60-90"]
    };

    let total_attempts = attempts.len() as u64;
    let mut progress = PacmanProgress::new(total_attempts, "Quantizing...");

    for (i, quality) in attempts.iter().enumerate() {
        if nerd {
            logger::nerd_cmd(&format!("pngquant --quality {} --force --output {} {}", quality, output, input));
        }
        
        let status = Command::new("pngquant")
            .arg("--quality").arg(quality)
            .arg("--force").arg("--output").arg(output).arg(input)
            .status()?;
        
        progress.set((i + 1) as u64);
        
        if !status.success() { return Err(anyhow!("pngquant failed.")); }
        
        let size = get_file_size_kb(output);
        if nerd {
            logger::nerd_result(&format!("Pass {}", i + 1), &format!("Quality {} -> {} KB", quality, size), i == attempts.len() - 1);
        }
        
        if let Some(target) = target_kb {
            if size <= target {
                progress.finish();
                return Ok(());
            }
        } else {
            progress.finish();
            return Ok(());
        }
    }
    
    progress.finish();

    // 2. Resize Consent Loop
    if let Some(target) = target_kb {
        let best_size = get_file_size_kb(output);
        
        if nerd {
            logger::nerd_stage(2, "Resize Required");
            logger::nerd_result("Current size", &format!("{} KB", best_size), false);
            logger::nerd_result("Target", &format!("{} KB", target), true);
        } else {
            println!("\n{}", "WARNING: Limit Reached!".yellow().bold());
            println!("   Smallest possible without resizing: {} KB (Target: {} KB)", 
                best_size.to_string().cyan(), target);
        }

        if !Confirm::new().with_prompt("   Resize image dimensions to fit?").default(false).interact()? {
            if !nerd {
                println!("   Keeping the {} KB version.", best_size);
            }
            return Ok(());
        }

        if nerd {
            logger::nerd_stage(3, "Dimension Scaling");
            logger::nerd_algo("ImageMagick resize", "O(n)", "Iterative 5% reduction");
        }

        let mut current_size = best_size;
        let mut scale = 95;
        
        let mut resize_progress = PacmanProgress::new(18, "Resizing...");

        while current_size > target && scale > 5 {
            if nerd {
                logger::nerd_cmd(&format!("magick {} -resize {}% {}", output, scale, output));
            }
            
            let status = Command::new("magick")
                .arg(output).arg("-resize").arg(format!("{}%", scale)).arg(output).status()?;
            if !status.success() { return Err(anyhow!("Resize failed.")); }

            current_size = get_file_size_kb(output);
            resize_progress.set(((95 - scale) / 5) as u64);
            
            if nerd {
                logger::nerd_result(&format!("Scale {}%", scale), &format!("{} KB", current_size), current_size <= target);
            }
            
            if current_size <= target {
                resize_progress.finish_with_message(&format!("Resized to {}%", scale));
                if nerd {
                    let elapsed = start.elapsed();
                    logger::nerd_result("Total time", &format!("{:.2}s", elapsed.as_secs_f64()), true);
                }
                return Ok(());
            }
            scale -= 5;
        }
        resize_progress.finish();
    }
    Ok(())
}

// PDF: Pure Binary Search (Optimal) with Floor Detection
fn compress_pdf(input: &str, output: &str, target_kb: Option<u64>, level: Option<CompressionLevel>, nerd: bool) -> Result<()> {
    let total_start = Instant::now();
    let original_size = get_file_size_kb(input);
    let mut gs_calls: u32 = 0;
    
    if target_kb.is_none() {
        let setting = match level {
            Some(CompressionLevel::Low) => "/printer",
            Some(CompressionLevel::Medium) => "/ebook",
            _ => "/ebook",
        };
        
        if nerd {
            logger::nerd_stage(1, "Preset Compression");
            logger::nerd_algo("Ghostscript", "O(1)", &format!("Preset: {}", setting));
        }
        
        let progress = PacmanProgress::new(1, "Compressing...");
        let result = run_gs(input, output, setting, None);
        progress.finish();
        return result;
    }

    let target = target_kb.unwrap();
    let temp_output = format!("{}.tmp", output);

    // Extreme compression warning
    let reduction_percent = if original_size > 0 && original_size > target {
        ((original_size - target) as f64 / original_size as f64 * 100.0) as u64
    } else { 0 };

    if reduction_percent > 95 {
        if nerd {
            logger::nerd_stage(0, "EXTREME COMPRESSION WARNING");
            logger::nerd_result("Reduction", &format!("{}%", reduction_percent), false);
            logger::nerd_result("Risk", "Output may be unreadable", true);
        } else {
            println!("\n{}", "WARNING: Extreme Compression!".yellow().bold());
            println!("   You're asking for {}% size reduction.", reduction_percent.to_string().red());
            println!("   {} KB -> {} KB may result in unreadable text/images.", original_size, target);
            println!("\n   We recommend a target of at least {} KB for readable output.", original_size / 10);
        }
        
        if !Confirm::new()
            .with_prompt("   Continue with extreme compression anyway?")
            .default(false)
            .interact()?
        {
            return Err(anyhow!("Compression cancelled by user."));
        }
        
        if nerd {
            println!("   |- User consented to extreme compression");
        }
    }

    // Floor check
    if nerd {
        logger::nerd_stage(1, "Floor Detection");
        logger::nerd_algo("Ghostscript /screen", "O(1)", "Finding minimum achievable size");
    }
    
    let mut progress = PacmanProgress::new(15, "Checking floor...");
    
    if run_gs(input, &temp_output, "/screen", None).is_ok() {
        gs_calls += 1;
        let floor_size = get_file_size_kb(&temp_output);
        
        if nerd {
            logger::nerd_result("Floor size", &format!("{} KB", floor_size), false);
            logger::nerd_result("Target", &format!("{} KB", target), true);
        }
        
        if floor_size > target {
            progress.finish_with_message("Floor > Target");
            
            if nerd {
                logger::nerd_stage(2, "TARGET UNREACHABLE");
                logger::nerd_result("Cause", "Incompressible content (fonts/vectors)", true);
            } else {
                println!("\n{}", "WARNING: Target Below Minimum!".yellow().bold());
                println!("   Original: {} KB", original_size);
                println!("   Smallest possible: {} KB", floor_size.to_string().cyan());
                println!("   Your target: {} KB", target.to_string().red());
                println!("\n   This PDF has fonts/vectors that can't be compressed further.");
            }
            
            if !Confirm::new()
                .with_prompt("   Save the smallest possible version?")
                .default(true)
                .interact()? 
            {
                let _ = fs::remove_file(&temp_output);
                return Err(anyhow!("Compression cancelled by user."));
            }
            
            fs::rename(&temp_output, output)?;
            
            if nerd {
                logger::nerd_final_result(72, original_size, floor_size, 1, 14, total_start.elapsed(), gs_calls, output);
            }
            return Ok(());
        }
        
        // Low quality warning
        let quality_headroom = if target > 0 { (target - floor_size) * 100 / target } else { 0 };
        
        if quality_headroom < 20 && floor_size > 50 {
            if nerd {
                logger::nerd_result("Headroom", &format!("{}% (low)", quality_headroom), true);
            } else {
                println!("\n{}", "WARNING: Low Quality!".yellow());
                println!("   To hit {} KB, output will be near minimum quality.", target);
                println!("   Recommended target: {} KB+ for better readability.", floor_size + (floor_size / 2));
            }
            
            if !Confirm::new()
                .with_prompt("   Continue anyway?")
                .default(true)
                .interact()?
            {
                let _ = fs::remove_file(&temp_output);
                return Err(anyhow!("Compression cancelled by user."));
            }
        }
    }
    
    let _ = fs::remove_file(&temp_output);
    progress.set(1);

    // Binary search
    if nerd {
        logger::nerd_stage(2, "Binary Search Optimization");
        logger::nerd_algo("Binary Search", "O(log n)", "Range: 1-2400 DPI | Max: 14 iterations");
    }
    
    let mut min_dpi: u64 = 1;
    let mut max_dpi: u64 = 2400;
    let mut best_dpi: u64 = 0;
    let mut best_size: u64 = 0;
    let mut found_valid = false;
    let max_iterations: u32 = 14;

    let mut attempts: u32 = 0;
    while min_dpi <= max_dpi && attempts < max_iterations {
        attempts += 1;
        let mid_dpi = (min_dpi + max_dpi) / 2;
        
        if nerd && attempts == 1 {
            logger::nerd_search_range(min_dpi, max_dpi, mid_dpi);
        }
        
        let iter_start = Instant::now();

        if run_gs(input, &temp_output, "/printer", Some(mid_dpi)).is_ok() {
            gs_calls += 1;
            let size = get_file_size_kb(&temp_output);
            let iter_time = iter_start.elapsed().as_millis();
            
            progress.set(attempts as u64 + 1);

            let action: String;
            if size <= target {
                fs::copy(&temp_output, output)?;
                found_valid = true;
                best_dpi = mid_dpi;
                best_size = size;
                action = format!("min={}", mid_dpi + 1);
                min_dpi = mid_dpi + 1;
            } else {
                action = format!("max={}", mid_dpi - 1);
                max_dpi = mid_dpi - 1;
            }
            
            if nerd {
                logger::nerd_attempt(attempts, max_iterations, mid_dpi, size, target, iter_time, &action);
            }
        }
    }

    let _ = fs::remove_file(&temp_output);
    progress.finish();

    if found_valid {
        if nerd {
            logger::nerd_final_result(best_dpi, original_size, best_size, attempts, max_iterations, total_start.elapsed(), gs_calls, output);
        } else if best_dpi < 50 {
            println!("\n{}", "   Note: Very low DPI - images may appear pixelated.".yellow());
        }
        Ok(())
    } else {
        if nerd {
            logger::nerd_result("Status", "FAILED - Could not reach target", true);
        } else {
            println!("   Warning: Could not reach target even at 1 DPI.");
        }
        run_gs(input, output, "/screen", None)
    }
}

// Helper to run GS
fn run_gs(input: &str, output: &str, setting: &str, dpi: Option<u64>) -> Result<()> {
    let mut cmd = Command::new("gs");
    cmd.arg("-sDEVICE=pdfwrite")
       .arg("-dCompatibilityLevel=1.4");

    if let Some(d) = dpi {
        // Custom DPI Mode
        cmd.arg("-dDownsampleColorImages=true")
           .arg(format!("-dColorImageResolution={}", d))
           .arg(format!("-dGrayImageResolution={}", d))
           .arg(format!("-dMonoImageResolution={}", d));
    } else {
        // Preset Mode
        cmd.arg(format!("-dPDFSETTINGS={}", setting));
    }

    cmd.arg("-dNOPAUSE").arg("-dQUIET").arg("-dBATCH")
       .arg(format!("-sOutputFile={}", output))
       .arg(input);

    let status = cmd.status()?;
    if !status.success() { return Err(anyhow!("Ghostscript failed.")); }
    Ok(())
}