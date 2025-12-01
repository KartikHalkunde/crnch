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

#[allow(dead_code)]
pub struct CompResult {
    pub algorithm: String,
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

pub fn compress_file(input: &str, output: &str, size_str: Option<String>, level: Option<CompressionLevel>, nerd: bool) -> Result<CompResult> {
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

// JPG: Smart Extent -> Fallbacks (My Version - Robust)
fn compress_jpg(input: &str, output: &str, target_kb: Option<u64>, level: Option<CompressionLevel>, nerd: bool) -> Result<CompResult> {
    let start = Instant::now();
    let progress = PacmanProgress::new(1, "Optimizing JPG...");
    let tmp_optim = format!("{}.jpegoptim.tmp.jpg", output);
    let original_size = get_file_size_kb(input);
    if let Some(target) = target_kb {
        if target >= original_size {
            println!("Requested size ({}) KB is larger than or equal to original file size ({} KB). No compression performed.", target, original_size);
            if Confirm::new().with_prompt("Keep original file?").default(true).interact()? {
                fs::copy(input, output)?;
                return Ok(CompResult{ algorithm: "No compression (requested size >= original)".to_string() });
            } else {
                return Err(anyhow!("Compression cancelled by user."));
            }
        }
    }

    // If no size flag, use standard preset
    if target_kb.is_none() {
        if nerd {
            use colored::*;
            println!("");
            println!("{}", "====================[ JPG COMPRESSION: NERD MODE ]====================".bold().yellow());
            println!("{}", "[INPUT]".bold().cyan());
            println!("   |- File: {}", input.cyan());
            println!("   |- Type: {}", "JPG".cyan());
            println!("   |- Size: {} KB", get_file_size_kb(input).to_string().green());
            println!("   '- Target: {}", "Auto (60-95% of original size)".green());
            println!("");
            println!("{}", "[STAGE 1] JPEG Lossless Optimization".bold().yellow());
            println!("   |- Tool: {}", "jpegoptim".cyan());
            println!("   |- Complexity: {}", "O(1)".green());
            println!("   |- Strategy: {}", "Stripping metadata and optimizing".green());
            println!("   |- Cmd: {}", format!("jpegoptim --strip-all --dest=. {}", input).cyan());
        }
        // Run jpegoptim for lossless optimization
        let status = Command::new("jpegoptim")
            .arg("--strip-all")
            .arg("--dest=.")
            .arg("--stdout")
            .arg(input)
            .stdout(fs::File::create(&tmp_optim)?)
            .status()?;
        if !status.success() {
            if nerd { logger::nerd_result("jpegoptim failed, skipping to magick stage", "", true); }
            // Fallback: use input directly for magick
            fs::copy(input, &tmp_optim)?;
        }
        let optim_size = get_file_size_kb(&tmp_optim);
        if nerd {
            println!("   |- Output Size after jpegoptim: {} KB", optim_size);
        }
        // Adaptive target compression: try 60%, then 65%, ..., up to 95% of original size
        let original_size = get_file_size_kb(input);
        let mut success = false;
        let mut final_size = original_size;
        let mut final_target = original_size;
        let mut tried_targets = Vec::new();
        for percent in [60, 65, 70, 75, 80, 85, 90, 95] {
            let target_kb = original_size * percent / 100;
            let try_out = if percent == 60 { output.to_string() } else { format!("{}.tgt{}p.jpg", output, percent) };
            if nerd {
                println!("");
                println!("{}", "[STAGE 2] JPEG Magick Compression".bold().yellow());
                println!("   |- Tool: {}", "ImageMagick".cyan());
                println!("   |- Complexity: {}", "O(1)".green());
                println!("   |- Strategy: {}", "Targeted lossy compression".green());
                println!("   |- Cmd: {}", format!("magick ... -define jpeg:extent={}KB -sampling-factor 4:4:4 -interlace Plane -strip {} {}", target_kb, &tmp_optim, &try_out).cyan());
                println!("   |- Target: {} KB ({}% of original)", target_kb.to_string().green(), percent.to_string().green());
            }
            let mut cmd = Command::new("magick");
            cmd.arg(&tmp_optim)
                .arg("-define").arg(format!("jpeg:extent={}KB", target_kb))
                .arg("-sampling-factor").arg("4:4:4")
                .arg("-interlace").arg("Plane")
                .arg("-strip")
                .arg(&try_out);
            let status = cmd.status()?;
            if !status.success() { continue; }
            let out_size = get_file_size_kb(&try_out);
            tried_targets.push(try_out.clone());
            if nerd {
                println!("   |- Result: {} KB {}", out_size, if out_size <= target_kb {"(Hit!)"} else {"(Miss)"});
            }
            if out_size <= target_kb {
                final_size = out_size;
                final_target = target_kb;
                success = true;
                // Move/copy to output if not already
                if try_out != output {
                    fs::copy(&try_out, output)?;
                }
                break;
            }
        }
        fs::remove_file(&tmp_optim).ok();
        // Clean up temp files except final output
        for f in tried_targets {
            if f != output { let _ = fs::remove_file(&f); }
        }
        progress.finish();
        let total_time = start.elapsed().as_secs_f64();
        if nerd {
            println!("");
            println!("{}", "====================[ JPG COMPRESSION RESULT ]====================".bold().yellow());
            println!("   |- Path: {}", output.cyan());
            println!("   |- Type: {}", "JPG".cyan());
            println!("   |- Original Size: {} KB", original_size.to_string().green());
            println!("   |- Final Size: {} KB", final_size.to_string().green());
            if success {
                let percent_reduced = 100.0 - (final_size as f64 / original_size as f64 * 100.0);
                println!("   |- Total Compression: {:.2}%", percent_reduced.to_string().green());
            } else {
                println!("   |- Total Compression: {}", "0% (could not reach any target)".red());
                println!("   |- Note: {}", "This image cannot be compressed to the desired size (60-95% of original). Keeping original.".red());
            }
            println!("   '- Total Time: {:.2}s", total_time.to_string().cyan());
        }
        if success {
            return Ok(CompResult{ algorithm: format!("jpegoptim + magick (Standard Preset, target {} KB)", final_target) });
        } else {
            // Inform user compression not possible
            println!("This image cannot be compressed to the desired size (60-95% of original). Keeping original.");
            fs::copy(input, output)?;
            return Ok(CompResult{ algorithm: "jpegoptim + magick (No reduction, original kept)".to_string() });
        }
    } else {
        // Original lossy/target logic for JPG compression
        if nerd {
            logger::nerd_stage(1, "JPEG Lossless Optimization");
            logger::nerd_result("Tool", "jpegoptim", false);
                logger::nerd_result("Complexity", "O(1)", false);
                logger::nerd_result("Strategy", "Stripping metadata and optimizing", false);
            logger::nerd_cmd(&format!("jpegoptim --strip-all --dest=. {}", input));
        }
        // Run jpegoptim for lossless optimization
        let status = Command::new("jpegoptim")
            .arg("--strip-all")
            .arg("--dest=.")
            .arg("--stdout")
            .arg(input)
            .stdout(fs::File::create(&tmp_optim)?)
            .status()?;
        if !status.success() {
            // If jpegoptim fails, fallback to magick directly
            if nerd { logger::nerd_result("jpegoptim failed, skipping to lossy stage", "", true); }
        }
        let optim_size = get_file_size_kb(&tmp_optim);
        if nerd {
            logger::nerd_result("Output Size after jpegoptim", &format!("{} KB", optim_size), false);
        }
        // If target met, use jpegoptim result
        if target_kb.is_some() && optim_size <= target_kb.unwrap() {
            fs::copy(&tmp_optim, output)?;
            fs::remove_file(&tmp_optim).ok();
            progress.finish();
            if nerd {
                let final_size = get_file_size_kb(output);
                let original_size = get_file_size_kb(input);
                let total_time = start.elapsed().as_secs_f64();
                println!("\n   =================================================================");
                println!("   OUTPUT IMAGE:");
                println!("   |- Path: {}", output);
                println!("   |- Type: JPG");
                println!("   |- Original Size: {} KB", original_size);
                println!("   |- Final Size: {} KB", final_size);
                println!("   |- Total Compression: {:.2}%", if original_size > 0 { (original_size - final_size) as f64 / original_size as f64 * 100.0 } else { 0.0 });
                println!("   '- Total Time: {:.2}s", total_time);
            }
            return Ok(CompResult{ algorithm: "jpegoptim (Lossless)".to_string() });
        }

        // Stage 2: Lossy compression with ImageMagick
        if nerd {
            logger::nerd_stage(2, "JPEG Lossy Compression");
            logger::nerd_result("Tool", "ImageMagick", false);
                logger::nerd_result("Complexity", "O(1)", false);
                logger::nerd_result("Strategy", "Smart extent targeting", false);
        }
        let mut cmd = Command::new("magick");
        cmd.arg(&tmp_optim).arg("-strip");
        cmd.arg("-sampling-factor").arg("4:4:4");

        if let Some(kb) = target_kb {
            let arg = format!("jpeg:extent={}KB", kb);
            cmd.arg("-define").arg(&arg);
            if nerd { logger::nerd_cmd(&format!("magick ... -define {}", arg)); }
        } else if let Some(lvl) = level {
            let q = match lvl {
                CompressionLevel::Low => "85",
                CompressionLevel::Medium => "75",
                CompressionLevel::High => "50",
            };
            cmd.arg("-quality").arg(q);
        } else {
            cmd.arg("-quality").arg("80");
        }

        cmd.arg(output);
        let status = cmd.status()?;
        fs::remove_file(&tmp_optim).ok();
        if !status.success() { return Err(anyhow!("ImageMagick failed.")); }
        progress.finish();

        // Check & Fallbacks
        if let Some(target) = target_kb {
            let current_size = get_file_size_kb(output);
            if nerd {
                let hit = if current_size <= target { "Hit!" } else { "Miss" };
                logger::nerd_result("Target", &format!("{} KB", target), false);
                logger::nerd_result("Result", &format!("{} KB ({})", current_size, hit), false);
            }
            if current_size > target {
                return handle_fallback_options(output, target, current_size, nerd, "JPG");
            }
        }

        if nerd {
            let final_size = get_file_size_kb(output);
            let original_size = get_file_size_kb(input);
            let total_time = start.elapsed().as_secs_f64();
            println!("\n   =================================================================");
            println!("   OUTPUT IMAGE:");
            println!("   |- Path: {}", output);
            println!("   |- Type: JPG");
            println!("   |- Original Size: {} KB", original_size);
            println!("   |- Final Size: {} KB", final_size);
            println!("   |- Total Compression: {:.2}%", if original_size > 0 { (original_size - final_size) as f64 / original_size as f64 * 100.0 } else { 0.0 });
            println!("   '- Total Time: {:.2}s", total_time);
        }
        return Ok(CompResult{ algorithm: "jpegoptim + ImageMagick".to_string() });
    }
}

// PNG: Waterfall Strategy (His Version - Smartest Logic)
fn compress_png(input: &str, output: &str, target_kb: Option<u64>, level: Option<CompressionLevel>, nerd: bool) -> Result<CompResult> {
    let _start = Instant::now(); // suppress unused variable warning
    let _level = level; // suppress unused variable warning
        let original_size = get_file_size_kb(input);
        if let Some(target) = target_kb {
            if target >= original_size {
                println!("Requested size ({}) KB is larger than or equal to original file size ({} KB). No compression performed.", target, original_size);
                if Confirm::new().with_prompt("Keep original file?").default(true).interact()? {
                    fs::copy(input, output)?;
                    return Ok(CompResult{ algorithm: "No compression (requested size >= original)".to_string() });
                } else {
                    return Err(anyhow!("Compression cancelled by user."));
                }
            }
        }
    // Removed unused nerd_stats variable
    
    // Use a single PacmanProgress bar for normal mode, always 100 steps
    let mut progress = if !nerd {
        Some(PacmanProgress::new(100, "Eating those bytes..."))
    } else {
        None
    };
    if nerd {
        logger::nerd_stage(1, "Stripping off Metadata");
        logger::nerd_result("Tool", "Oxipng", false);
        logger::nerd_result("Stratergy", "Removing metadata from the image (lossless)", false);
        logger::nerd_result("Original Size", &format!("{} KB", original_size), false);
        logger::nerd_cmd(&format!("oxipng -o 2 --strip safe --quiet --out {} {}", output, input));
    }
    let oxi_out = format!("{}.oxipng.tmp.png", output);
    let oxi_status = Command::new("oxipng")
        .arg("-o").arg("2").arg("--strip").arg("safe").arg("--quiet")
        .arg("--out").arg(&oxi_out).arg(input)
        .status()?;
    // No progress bar update here; only animate in the lossless branch below
    if nerd {
        let oxi_size = get_file_size_kb(&oxi_out);
        let meta_removed = if oxi_size < original_size { original_size - oxi_size } else { 0 };
        logger::nerd_result("Metadata Removed", &format!("{} KB", meta_removed), false);
        logger::nerd_result("Output Size after oxipng", &format!("{} KB", oxi_size), false);
        let reduction = if original_size > 0 { (original_size - oxi_size) as f64 / original_size as f64 * 100.0 } else { 0.0 };
        logger::nerd_result("Reduction", &format!("{:.2}%", reduction), true);
    }
    let oxi_size = get_file_size_kb(&oxi_out);

    // If no target, return lossless result with smooth Pacman bar
    if target_kb.is_none() {
        if let Some(ref mut bar) = progress {
            for i in 1..=100 {
                bar.set(i);
                std::thread::sleep(std::time::Duration::from_millis(8));
            }
            bar.finish();
        }
        fs::copy(&oxi_out, output)?;
        fs::remove_file(&oxi_out).ok();
        return Ok(CompResult{ algorithm: "oxipng (Lossless)".to_string() });
    }

    let target = target_kb.unwrap();
    if oxi_size <= target {
        fs::copy(&oxi_out, output)?;
        fs::remove_file(&oxi_out).ok();
        if nerd { logger::nerd_result("Result", "Target hit losslessly!", true); }
        return Ok(CompResult{ algorithm: "oxipng (Lossless)".to_string() });
    }

    // 2. COLOR QUANTIZATION (Binary Search on Quality Index)
    if nerd {
        logger::nerd_stage(2, "Color Quantization");
        logger::nerd_result("Tool", "pngquant", false);
        logger::nerd_result("Strategy", "Color Quantization using Binary search for quality index 30-100(lossy)", false);
        logger::nerd_result("Complexity", "O(log n)", false);
        logger::nerd_cmd(&format!("pngquant --quality 30-100 --force --output <out> <in>"));
        let color_check = if oxi_size < original_size * 95 / 100 { "Likely Color" } else { "Likely BW" };
        logger::nerd_result("Color Check Result", color_check, false);
    }
    let mut min_q = 30;
    let mut max_q = 100;
    let mut best_candidate: Option<(u8, u64)> = None;
    let pq_out = format!("{}.pngquant.tmp.png", output);
    let mut attempts = 0;
    // Color quantization
    while min_q <= max_q && attempts < 8 {
        attempts += 1;
        let mid_q = (min_q + max_q) / 2;
        let t0 = Instant::now();
        let status = Command::new("pngquant")
            .arg("--quality").arg(format!("{}-{}", mid_q, max_q))
            .arg("--force").arg("--output").arg(&pq_out).arg(&oxi_out)
            .status()?;
        let elapsed_ms = t0.elapsed().as_millis();
        if !status.success() {
            max_q = mid_q - 1;
            continue;
        }
        let pq_size = get_file_size_kb(&pq_out);
        let delta = if pq_size > target { pq_size - target } else { 0 };
        if nerd {
            logger::nerd_result(&format!("[{}] Quality {}% -> {} KB [{}] (+{} KB) | {}ms | next: {}", attempts, mid_q, pq_size, if pq_size <= target { "OK" } else { "XX" }, delta, elapsed_ms, if pq_size <= target { "min=mid+1" } else { "max=mid-1" }), "", false);
        }
        if pq_size <= target {
            best_candidate = Some((mid_q as u8, pq_size));
            min_q = mid_q + 1; // Try higher quality
        } else {
            if mid_q == 30 {
                if nerd {
                    logger::nerd_result("quality floor reached in pngquant, cannot compress further:", "", true);
                }
            }
            max_q = mid_q - 1; // Try lower quality
        }
    }
    if let Some(ref mut bar) = progress {
        for i in 26..=50 {
            bar.set(i);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    // If we found a good quantization, use it
    let mut color_candidate_path: Option<String> = None;
    if let Some((q, _)) = best_candidate {
        fs::copy(&pq_out, output)?;
        fs::remove_file(&pq_out).ok();
        fs::remove_file(&oxi_out).ok();
        
        // Polish
        let _ = Command::new("oxipng").arg("-o").arg("2").arg("--strip").arg("safe").arg("--quiet").arg(output).status();
        if nerd { logger::nerd_result("Optimal Quality", &q.to_string(), true); }
        return Ok(CompResult{ algorithm: "Hybrid (Oxipng + Binary Search)".to_string() });
    } else {
        // Keep track of the last attempt (best effort color)
        color_candidate_path = Some(pq_out.clone());
    }

    // 3. GRAYSCALE (XEROX MODE)
    let gray_out = format!("{}.gray.tmp.png", output);
    if nerd {
        let color_check = if oxi_size < original_size * 95 / 100 { "Likely Color" } else { "Likely BW" };
        logger::nerd_stage(3, "Grayscale Conversion");
        if color_check == "Likely BW" {
            logger::nerd_result("Tool", "magick", false);
            logger::nerd_result("Strategy", "Convert to grayscale", false);
            logger::nerd_result("Complexity", "O(1)", false);
        } else {
            logger::nerd_result("grayscale conversion not required for this image.:", "", true);
        }
        println!(""); // Add blank line after stage 3 and warning
    }
    let _gray_status = Command::new("magick")
        .arg(&oxi_out).arg("-colorspace").arg("Gray").arg("-depth").arg("8").arg(&gray_out)
        .status()?;
    let gray_size = get_file_size_kb(&gray_out);

    // Branch A: Grayscale fits
    if gray_size <= target {
        if Confirm::new().with_prompt(format!("Target reached by converting to Grayscale ({} KB). Proceed?", gray_size)).default(true).interact()? {
            fs::copy(&gray_out, output)?;
            // Cleanup
            fs::remove_file(&gray_out).ok();
            fs::remove_file(&oxi_out).ok();
            if let Some(ref p) = color_candidate_path { fs::remove_file(p).ok(); }
            if nerd { logger::nerd_result("Result", "Converted to Grayscale", true); }
            return Ok(CompResult{ algorithm: "pngquant + Grayscale".to_string() });
        }
    }

    // Branch B: Grayscale Fails OR User Rejected
    let mut resize_input = &oxi_out;
    let mut mode_str = "Color";

    if gray_size < oxi_size {
        // Grayscale is smaller, offer it as base for resizing
        if Confirm::new().with_prompt("Target unreachable in Color. Proceed with Grayscale Resizing?").default(true).interact()? {
            resize_input = &gray_out;
            mode_str = "Grayscale";
        }
    } else {
        // Automatically proceed to resizing without warning or prompt
    }

    // 4. RESIZE LOOP
    if nerd {
        logger::nerd_stage(4, "Image Resizing");
        logger::nerd_result("Tool", "magick", false);
        logger::nerd_result("Strategy", "Resizing image dimentions using Binary search as Scale index(too lossy)", false);
        logger::nerd_result("Complexity", "O(log n)", false);
        logger::nerd_cmd(&format!("magick <in> -resize <scale>% <out>"));
    }
    let mut min_scale = 1;
    let mut max_scale = 100;
    let mut best_scale: Option<(u8, u64)> = None;
    let resize_out = format!("{}.resize.tmp.png", output);
    let mut attempts = 0;
    while min_scale <= max_scale && attempts < 8 {
        attempts += 1;
        let mid_scale = (min_scale + max_scale) / 2;
        let t0 = Instant::now();
        let status = Command::new("magick")
            .arg(resize_input)
            .arg("-resize").arg(format!("{}%", mid_scale))
            .arg(&resize_out).status()?;
        let elapsed_ms = t0.elapsed().as_millis();
        if status.success() {
            let size = get_file_size_kb(&resize_out);
            let delta = size as i64 - target as i64;
            if nerd {
                let sign = if delta >= 0 { "+" } else { "-" };
                logger::nerd_result(&format!(
                    "[{}] Scale {}% -> {} KB [{}] ({}{} KB) | {}ms | next: {}",
                    attempts,
                    mid_scale,
                    size,
                    if size <= target { "OK" } else { "XX" },
                    sign,
                    delta.abs(),
                    elapsed_ms,
                    if size <= target { "min=mid+1" } else { "max=mid-1" }
                ), "", false);
            }
            if size <= target {
                best_scale = Some((mid_scale as u8, size));
                min_scale = mid_scale + 1; // Try larger
            } else {
                max_scale = mid_scale - 1;
            }
        }
    }
    if let Some(ref mut bar) = progress {
        for i in 51..=99 {
            bar.set(i);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        bar.set(100);
        bar.finish();
    }
    let mut final_size = 0;
    if let Some((scale, size)) = best_scale {
        fs::copy(&resize_out, output)?;
        final_size = size;
        if nerd { logger::nerd_result("Resize fits target", &format!("{}%", scale), true); }
        // Final Polish
        let _ = Command::new("oxipng").arg("-o").arg("2").arg("--strip").arg("safe").arg("--quiet").arg(output).status();
    } else {
        // Impossible
        if Confirm::new().with_prompt("Target unreachable. Save smallest possible?").default(true).interact()? {
            final_size = get_file_size_kb(&resize_out);
            fs::copy(&resize_out, output)?;
        }
    }
    // Cleanup
    fs::remove_file(&oxi_out).ok();
    fs::remove_file(&gray_out).ok();
    fs::remove_file(&resize_out).ok();
    if let Some(ref p) = color_candidate_path { fs::remove_file(p).ok(); }
    if nerd {
        let total_time = _start.elapsed().as_secs_f64();
        println!("\n   =================================================================");
        println!("   OUTPUT IMAGE:");
        println!("   |- Path: {}", output);
        println!("   |- Type: PNG");
        println!("   |- Original Size: {} KB", original_size);
        println!("   |- Final Size: {} KB", final_size);
        println!("   |- Total Compression: {:.2}%", if original_size > 0 { (original_size - final_size) as f64 / original_size as f64 * 100.0 } else { 0.0 });
        println!("   '- Total Time: {:.2}s", total_time);
    }
    Ok(CompResult{ algorithm: "Hybrid Chain".to_string() })
}

// PDF: Binary Search (Optimal) with Floor Detection
fn compress_pdf(input: &str, output: &str, target_kb: Option<u64>, level: Option<CompressionLevel>, nerd: bool) -> Result<CompResult> {
    let total_start = Instant::now();
    let original_size = get_file_size_kb(input);
    let mut _gs_calls: u32 = 0;
    if let Some(target) = target_kb {
        if target >= original_size {
            println!("Requested size ({}) KB is larger than or equal to original file size ({} KB). No compression performed.", target, original_size);
            if Confirm::new().with_prompt("Keep original file?").default(true).interact()? {
                fs::copy(input, output)?;
                return Ok(CompResult{ algorithm: "No compression (requested size >= original)".to_string() });
            } else {
                return Err(anyhow!("Compression cancelled by user."));
            }
        }
    }

    if target_kb.is_none() {
        // Use standard compression (Ghostscript /printer preset)
        let setting = "/printer";
        if nerd {
            logger::nerd_stage(1, "Standard Compression");
            logger::nerd_algo("Ghostscript", "O(1)", &format!("Preset: {}", setting));
        }
        let progress = PacmanProgress::new(1, "Compressing...");
        run_gs(input, output, setting, None)?;
        progress.finish();
        return Ok(CompResult{ algorithm: format!("Standard Compression ({})", setting) });
    }

    let target = target_kb.unwrap();
    let temp_output = format!("{}.tmp", output);

    // Stage 1: Floor Detection
    let mut floor_size = 0;
    let mut floor_checked = false;
    if nerd {
        logger::nerd_stage(1, "Floor Detection");
        logger::nerd_result("Tool", "Ghostscript", false);
        logger::nerd_result("Strategy", "PDF minimum size calculation using /screen preset", false);
    }
    if run_gs(input, &temp_output, "/screen", None).is_ok() {
        _gs_calls += 1;
        floor_size = get_file_size_kb(&temp_output);
        floor_checked = true;
        if nerd {
            if floor_size > target {
                logger::nerd_result("Status", "Floor > Target (cannot be compressed to the desired target)", true);
            } else {
                logger::nerd_result("Status", "Floor < Target (size reduction possible)", true);
            }
        }
    }

    if floor_checked && floor_size > target {
        let progress = PacmanProgress::new(1, "Floor > Target");
        progress.finish_with_message("Floor > Target");
        if nerd {
            println!("\n{}", "WARNING: Target Below Minimum!".yellow().bold());
            println!("   Smallest possible: {} KB", floor_size.to_string().cyan());
            println!("   Your target: {} KB", target.to_string().red());
            println!("   Best possible output near target is: {} KB", floor_size.to_string().green());
            println!("WARNING: Could not reach target size without destroying quality.");
        }
        if !Confirm::new().with_prompt("   Save the smallest possible version?").default(true).interact()? {
            let _ = fs::remove_file(&temp_output);
            return Err(anyhow!("Compression cancelled."));
        }
        fs::rename(&temp_output, output)?;
        println!("Tip: Could not reach target size without destroying quality.\n   Try a higher size.");
        return Ok(CompResult{ algorithm: "Floor (Min Quality)".to_string() });
    }
    if nerd {
        logger::nerd_stage(2, "Size Reduction");
        logger::nerd_result("Tool", "Ghostscript", false);
        logger::nerd_result("Strategy", "PDF compression using Binary search with range: 1-2400 DPI", false);
        logger::nerd_result("Complexity", "O(log n)", false);
        logger::nerd_cmd("gs ... -dColorImageResolution=<dpi> ...");
    }
    let mut min_dpi: u64 = 1;
    let mut max_dpi: u64 = 2400;
    let mut best_dpi: u64 = 0;
    let mut best_size: u64 = 0;
    let mut found_valid = false;
    let max_iterations: u32 = 14;
    let mut attempts: u32 = 0;
    let mut search_progress = PacmanProgress::new(14, "Optimizing DPI...");
    while min_dpi <= max_dpi && attempts < max_iterations {
        attempts += 1;
        let mid_dpi = (min_dpi + max_dpi) / 2;
        if nerd && attempts == 1 {
            logger::nerd_search_range(min_dpi, max_dpi, mid_dpi);
        }
        let iter_start = Instant::now();
        if run_gs(input, &temp_output, "/printer", Some(mid_dpi)).is_ok() {
            _gs_calls += 1;
            let size = get_file_size_kb(&temp_output);
            search_progress.set(attempts as u64 + 1);
            let action_str = if size <= target { "min=mid+1" } else { "max=mid-1" };
            if nerd {
                logger::nerd_attempt(attempts as u32, 14, mid_dpi, size, target, iter_start.elapsed().as_millis(), action_str);
            }
            if size <= target {
                fs::copy(&temp_output, output)?;
                found_valid = true;
                best_dpi = mid_dpi;
                best_size = size;
                min_dpi = mid_dpi + 1;
            } else {
                max_dpi = mid_dpi - 1;
            }
        }
    }
    let _ = fs::remove_file(&temp_output);
    search_progress.finish();
    if found_valid {
        if nerd {
            let total_time = total_start.elapsed().as_secs_f64();
            println!("\n   =================================================================");
            println!("   OUTPUT IMAGE:");
            println!("   |- Path: {}", output);
            println!("   |- Type: PDF");
            println!("   |- Original Size: {} KB", original_size);
            println!("   |- Final Size: {} KB", best_size);
            println!("   |- Total Compression: {:.2}%", if original_size > 0 { (original_size - best_size) as f64 / original_size as f64 * 100.0 } else { 0.0 });
            println!("   '- Total Time: {:.2}s", total_time);
        } else if best_dpi < 50 {
            println!("\n{}", "   Note: Very low DPI - images may appear pixelated.".yellow());
        }
        Ok(CompResult{ algorithm: format!("Binary Search ({} DPI)", best_dpi) })
    } else {
        run_gs(input, output, "/screen", None)?;
        Ok(CompResult{ algorithm: "Fallback /screen".to_string() })
    }
}

// ==================== SHARED FALLBACK LOGIC ====================

fn handle_fallback_options(output: &str, target: u64, current_size: u64, nerd: bool, format: &str) -> Result<CompResult> {
    println!("\n{}", "WARNING: Limit Reached!".yellow().bold());
    println!("   Smallest size without resizing: {} KB (Target: {} KB)", current_size.to_string().cyan(), target);

    // Option 1: Grayscale
    if Confirm::new().with_prompt("   Convert to Grayscale (B&W) to save space?").default(true).interact()? {
        if nerd { logger::nerd_stage(3, "Grayscale Conversion"); }
        let progress = PacmanProgress::new(1, "Desaturating...");
        
        let status = Command::new("magick")
            .arg(output).arg("-colorspace").arg("Gray").arg("-depth").arg("8").arg(output).status()?;
        
        progress.finish();
        
        if status.success() {
            let gray_size = get_file_size_kb(output);
            if gray_size <= target {
                println!("   ✨ Grayscale worked! ({} KB)", gray_size);
                return Ok(CompResult{ algorithm: format!("{} + Grayscale", format) });
            } else {
                if nerd { logger::nerd_result("Grayscale size", &format!("{} KB (Still > Target)", gray_size), true); }
            }
        }
    }

    // Option 2: Brutal Resize
    if Confirm::new().with_prompt("   Resize image dimensions to fit?").default(false).interact()? {
        if nerd { logger::nerd_stage(4, "Dimension Scaling (Binary Search)"); }
        println!("   Resizing image to fit...");
        
        let mut min_scale = 1;
        let mut max_scale = 99;
        let mut best_scale = 0;
        let mut attempts = 0;
        let mut progress = PacmanProgress::new(8, "Scaling...");

        while min_scale <= max_scale && attempts < 8 {
            attempts += 1;
            progress.set(attempts);
            let mid_scale = (min_scale + max_scale) / 2;

            let status = Command::new("magick")
                .arg(output).arg("-resize").arg(format!("{}%", mid_scale)).arg(output).status()?;

            if status.success() {
                let size = get_file_size_kb(output);
                if nerd {
                    logger::nerd_result(&format!("Scale {}%", mid_scale), &format!("{} KB", size), size <= target);
                }

                if size <= target {
                    best_scale = mid_scale;
                    min_scale = mid_scale + 1; 
                } else {
                    max_scale = mid_scale - 1;
                }
            }
        }
        progress.finish();

        if best_scale > 0 {
            Command::new("magick").arg(output).arg("-resize").arg(format!("{}%", best_scale)).arg(output).status()?;
            println!("   Resized to {}% scale.", best_scale);
            return Ok(CompResult{ algorithm: format!("{} + Resize {}%", format, best_scale) });
        }
    }

    println!("   Keeping the {} KB version.", get_file_size_kb(output));
    Ok(CompResult{ algorithm: "Best Effort".to_string() })
}

fn run_gs(input: &str, output: &str, setting: &str, dpi: Option<u64>) -> Result<()> {
    let mut cmd = Command::new("gs");
    cmd.arg("-sDEVICE=pdfwrite")
        .arg("-dCompatibilityLevel=1.4")
        .arg("-dCompressFonts=true")
        .arg("-dSubsetFonts=true");
    if let Some(d) = dpi {
        cmd.arg("-dDownsampleColorImages=true")
           .arg(format!("-dColorImageResolution={}", d))
           .arg(format!("-dGrayImageResolution={}", d))
           .arg(format!("-dMonoImageResolution={}", d));
    } else {
        cmd.arg(format!("-dPDFSETTINGS={}", setting));
    }
    cmd.arg("-dNOPAUSE").arg("-dQUIET").arg("-dBATCH")
       .arg(format!("-sOutputFile={}", output)).arg(input);
    let status = cmd.status()?;
    if !status.success() { return Err(anyhow!("Ghostscript failed.")); }
    Ok(())
}