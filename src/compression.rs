use std::process::Command;
use std::path::Path;
use anyhow::{Result, anyhow};
use clap::ValueEnum;
use std::fs;
use std::time::Instant;
use dialoguer::Confirm;
use colored::*;
use crate::logger::{self, PacmanProgress};
use crate::utils;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum CompressionLevel {
    Low,    // Better Quality
    Medium, // Balanced
    High,   // Smallest size
}

pub struct CompResult {
    pub algorithm: String,
    pub time_ms: u128,
}

/// RAII helper for temp files - automatically cleans up on drop
#[allow(dead_code)]
struct TempFile {
    path: String,
    keep: bool,
}

#[allow(dead_code)]
impl TempFile {
    fn new(path: String) -> Self {
        TempFile { path, keep: false }
    }
    
    fn path(&self) -> &str {
        &self.path
    }
    
    /// Mark file to be kept (not deleted on drop)
    fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Generate a unique temp file path using PID
#[allow(dead_code)]
fn temp_path(base: &str, suffix: &str) -> String {
    format!("{}.{}.tmp.{}", base, std::process::id(), suffix)
}

fn get_file_size_kb(path: &str) -> u64 {
    fs::metadata(path).map(|m| m.len() / 1024).unwrap_or(0)
}

/// Helper to create CompResult with timing from a start instant
fn result_with_time(algorithm: impl Into<String>, start: Instant) -> CompResult {
    CompResult {
        algorithm: algorithm.into(),
        time_ms: start.elapsed().as_millis(),
    }
}

pub fn compress_file(input: &str, output: &str, size_str: Option<String>, level: Option<CompressionLevel>, nerd: bool, auto_yes: bool) -> Result<CompResult> {
    let path = Path::new(input);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let target_kb = if let Some(s) = size_str { utils::parse_size(&s) } else { None };

    match ext.as_str() {
        "jpg" | "jpeg" => compress_jpg(input, output, target_kb, level, nerd, auto_yes),
        "png" => compress_png(input, output, target_kb, level, nerd, auto_yes),
        "pdf" => compress_pdf(input, output, target_kb, level, nerd, auto_yes),
        _ => Err(anyhow!("Unsupported file type: .{}", ext)),
    }
}

// ---------------------- ENGINES ----------------------

// JPG: Smart Extent -> Fallbacks (My Version - Robust)
fn compress_jpg(input: &str, output: &str, target_kb: Option<u64>, level: Option<CompressionLevel>, nerd: bool, auto_yes: bool) -> Result<CompResult> {
    let start = Instant::now();
    let progress = PacmanProgress::new(1, "Optimizing JPG...");
    let tmp_optim = format!("{}.jpegoptim.tmp.jpg", output);
    let original_size = get_file_size_kb(input);
    if let Some(target) = target_kb {
        if target >= original_size {
            println!("Requested size ({}) KB is larger than or equal to original file size ({} KB). No compression performed.", target, original_size);
            let should_keep = if auto_yes {
                if nerd { println!("   [Auto-yes enabled, keeping original]"); }
                true
            } else {
                Confirm::new().with_prompt("Keep original file?").default(true).interact()?
            };
            if should_keep {
                fs::copy(input, output)?;
                return Ok(result_with_time("No compression (requested size >= original)", start));
            } else {
                return Err(anyhow!("Compression cancelled by user."));
            }
        }
    }

    // If no size flag, use standard preset
    if target_kb.is_none() {
        if nerd {
            logger::nerd_stage(1, "JPEG Lossless Optimization");
            logger::nerd_result("Tool", "jpegoptim", false);
            logger::nerd_result("Complexity", "O(n) I/O bound", false);
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
            .stderr(if nerd { std::process::Stdio::inherit() } else { std::process::Stdio::null() })
            .status()?;
        if !status.success() {
            if nerd { logger::nerd_result("Status", "jpegoptim failed, skipping to magick stage", true); }
            // Fallback: use input directly for magick
            fs::copy(input, &tmp_optim)?;
        }
        let optim_size = get_file_size_kb(&tmp_optim);
        if nerd {
            logger::nerd_result("Output Size", &format!("{} KB", optim_size), true);
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
                logger::nerd_stage(2, "JPEG Lossy Compression");
                logger::nerd_result("Tool", "ImageMagick", false);
                logger::nerd_result("Complexity", "O(n) I/O bound", false);
                logger::nerd_result("Strategy", "Targeted lossy compression", false);
                logger::nerd_result("Target", &format!("{} KB ({}% of original)", target_kb, percent), false);
                logger::nerd_cmd(&format!("magick ... -define jpeg:extent={}KB -sampling-factor 4:4:4 -interlace Plane -strip {} {}", target_kb, &tmp_optim, &try_out));
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
                let hit_miss = if out_size <= target_kb {"Hit!"} else {"Miss"};
                logger::nerd_result("Result", &format!("{} KB ({})", out_size, hit_miss), true);
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
            logger::nerd_output_summary(input, output, original_size, final_size, "jpegoptim + magick (Standard Preset)", total_time);
        }
        if success {
            Ok(result_with_time(format!("jpegoptim + magick (Standard Preset, target {} KB)", final_target), start))
        } else {
            // Inform user compression not possible
            println!("This image cannot be compressed to the desired size (60-95% of original). Keeping original.");
            fs::copy(input, output)?;
            Ok(result_with_time("jpegoptim + magick (No reduction, original kept)", start))
        }
    } else {
        // Original lossy/target logic for JPG compression
        if nerd {
            logger::nerd_stage(1, "JPEG Lossless Optimization");
            logger::nerd_result("Tool", "jpegoptim", false);
                logger::nerd_result("Complexity", "O(n) I/O bound", false);
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
            .stderr(if nerd { std::process::Stdio::inherit() } else { std::process::Stdio::null() })
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
        if let Some(target) = target_kb {
            if optim_size <= target {
                fs::copy(&tmp_optim, output)?;
                fs::remove_file(&tmp_optim).ok();
                progress.finish();
                if nerd {
                    let original_size = get_file_size_kb(input);
                    let final_size = get_file_size_kb(output);
                    let total_time = start.elapsed().as_secs_f64();
                    logger::nerd_output_summary(input, output, original_size, final_size, "jpegoptim (Lossless)", total_time);
                }
                return Ok(result_with_time("jpegoptim (Lossless)", start));
            }
        }

        // Stage 2: Lossy compression with ImageMagick
        if nerd {
            logger::nerd_stage(2, "JPEG Lossy Compression");
            logger::nerd_result("Tool", "ImageMagick", false);
                logger::nerd_result("Complexity", "O(n) I/O bound", false);
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
                logger::nerd_result("Result", &format!("{} KB ({})", current_size, hit), true);
            }
            if current_size > target {
                let fallback_result = handle_fallback_options(output, target, current_size, nerd, "JPG");
                if nerd {
                    let final_size = get_file_size_kb(output);
                    let original_size = get_file_size_kb(input);
                    let total_time = start.elapsed().as_secs_f64();
                    logger::nerd_output_summary(input, output, original_size, final_size, "jpegoptim + ImageMagick", total_time);
                }
                return fallback_result;
            }
        }

        if nerd {
            let final_size = get_file_size_kb(output);
            let original_size = get_file_size_kb(input);
            let total_time = start.elapsed().as_secs_f64();
            logger::nerd_output_summary(input, output, original_size, final_size, "jpegoptim + ImageMagick", total_time);
        }
        Ok(result_with_time("jpegoptim + ImageMagick", start))
    }
}

// PNG: Waterfall Strategy (His Version - Smartest Logic)
fn compress_png(input: &str, output: &str, target_kb: Option<u64>, _level: Option<CompressionLevel>, nerd: bool, auto_yes: bool) -> Result<CompResult> {
    let start = Instant::now();
    let original_size = get_file_size_kb(input);
    if let Some(target) = target_kb {
        if target >= original_size {
            println!("Requested size ({}) KB is larger than or equal to original file size ({} KB). No compression performed.", target, original_size);
            let should_keep = if auto_yes {
                if nerd { println!("   [Auto-yes enabled, keeping original]"); }
                true
            } else {
                Confirm::new().with_prompt("Keep original file?").default(true).interact()?
            };
            if should_keep {
                fs::copy(input, output)?;
                return Ok(result_with_time("No compression (requested size >= original)", start));
            } else {
                return Err(anyhow!("Compression cancelled by user."));
            }
        }
    }

    // Use a single PacmanProgress bar for normal mode, always 100 steps
    let mut progress = if !nerd {
        Some(PacmanProgress::new(100, "Eating those bytes..."))
    } else {
        None
    };
    if nerd {
        logger::nerd_stage(1, "Stripping off Metadata");
        logger::nerd_result("Tool", "Oxipng", false);
        logger::nerd_result("Strategy", "Removing metadata from the image (lossless)", false);
        logger::nerd_result("Original Size", &format!("{} KB", original_size), false);
        logger::nerd_cmd(&format!("oxipng -o 2 --strip safe --quiet --out {} {}", output, input));
    }
    let oxi_out = format!("{}.oxipng.tmp.png", output);
    let _oxi_status = Command::new("oxipng")
        .arg("-o").arg("2").arg("--strip").arg("safe").arg("--quiet")
        .arg("--out").arg(&oxi_out).arg(input)
        .status()?;
    // No progress bar update here; only animate in the lossless branch below
    if nerd {
        let oxi_size = get_file_size_kb(&oxi_out);
        let meta_removed = original_size.saturating_sub(oxi_size);
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
        if nerd {
            let total_time = start.elapsed().as_secs_f64();
            let final_size = get_file_size_kb(output);
            logger::nerd_output_summary(input, output, original_size, final_size, "oxipng (Lossless)", total_time);
        }
        return Ok(result_with_time("oxipng (Lossless)", start));
    }

    let target = target_kb.unwrap();
    if oxi_size <= target {
        fs::copy(&oxi_out, output)?;
        fs::remove_file(&oxi_out).ok();
        if nerd {
            logger::nerd_result("Result", "Target hit losslessly!", true);
            let total_time = start.elapsed().as_secs_f64();
            let final_size = get_file_size_kb(output);
            logger::nerd_output_summary(input, output, original_size, final_size, "oxipng (Lossless)", total_time);
        }
        return Ok(result_with_time("oxipng (Lossless)", start));
    }

    // 2. COLOR QUANTIZATION (Binary Search on Quality Index)
    if nerd {
        logger::nerd_stage(2, "Color Quantization");
        logger::nerd_result("Tool", "pngquant", false);
        logger::nerd_result("Strategy", "Color Quantization using Binary search for quality index 30-100(lossy)", false);
        logger::nerd_result("Complexity", "O(log n)", false);
        logger::nerd_cmd(&format!("pngquant --quality 30-100 --force --output {} {}", output, &oxi_out));
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
        let action = if pq_size <= target { "min=mid+1" } else { "max=mid-1" };
        if nerd {
            logger::nerd_quality_attempt(attempts, 8, mid_q as u8, pq_size, target, elapsed_ms, action);
        }
        if pq_size <= target {
            best_candidate = Some((mid_q as u8, pq_size));
            min_q = mid_q + 1; // Try higher quality
        } else {
            if mid_q == 30
                && nerd {
                    logger::nerd_result("quality floor reached in pngquant, cannot compress further:", "", true);
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
    let _color_candidate_path: Option<String>;
    if let Some((q, _)) = best_candidate {
        fs::copy(&pq_out, output)?;
        fs::remove_file(&pq_out).ok();
        fs::remove_file(&oxi_out).ok();
        
        // Polish
        let _ = Command::new("oxipng").arg("-o").arg("2").arg("--strip").arg("safe").arg("--quiet").arg(output).status();
        if let Some(ref mut bar) = progress {
            bar.set(100);
            bar.finish();
        }
        if nerd {
            logger::nerd_result("Optimal Quality", &q.to_string(), true);
            let total_time = start.elapsed().as_secs_f64();
            let final_size = get_file_size_kb(output);
            logger::nerd_output_summary(input, output, original_size, final_size, "Hybrid (Oxipng + Binary Search)", total_time);
        }
        return Ok(result_with_time("Hybrid (Oxipng + Binary Search)", start));
    } else {
        // Keep track of the last attempt (best effort color)
        _color_candidate_path = Some(pq_out.clone());
    }

    // 3. GRAYSCALE (XEROX MODE)
    let gray_out = format!("{}.gray.tmp.png", output);
    if nerd {
        let color_check = if oxi_size < original_size * 95 / 100 { "Likely Color" } else { "Likely BW" };
        logger::nerd_stage(3, "Grayscale Conversion");
        if color_check == "Likely BW" {
            logger::nerd_result("Tool", "magick", false);
            logger::nerd_result("Strategy", "Convert to grayscale", false);
            logger::nerd_result("Complexity", "O(n) I/O bound", false);
        } else {
            logger::nerd_result("grayscale conversion not required for this image.:", "", true);
        }
        println!(); // Add blank line after stage 3 and warning
    }
    let _gray_status = Command::new("magick")
        .arg(&oxi_out).arg("-colorspace").arg("Gray").arg("-depth").arg("8").arg(&gray_out)
        .status()?;
    let gray_size = get_file_size_kb(&gray_out);

    // Branch A: Grayscale fits
    if gray_size <= target {
        if let Some(ref mut bar) = progress {
            bar.set(100);
            bar.finish();
        }
        progress = None; // Clear progress bar reference
        let should_grayscale = if auto_yes {
            if nerd { println!("   [Auto-yes enabled, converting to grayscale]"); }
            true
        } else {
            Confirm::new().with_prompt(format!("Target reached by converting to Grayscale ({} KB). Proceed?", gray_size)).default(true).interact()?
        };
        if should_grayscale {
            fs::copy(&gray_out, output)?;
            // Cleanup
            fs::remove_file(&gray_out).ok();
            fs::remove_file(&oxi_out).ok();
            if let Some(ref p) = _color_candidate_path { fs::remove_file(p).ok(); }
            if nerd { logger::nerd_result("Result", "Converted to Grayscale", true); }
            if nerd {
                let total_time = start.elapsed().as_secs_f64();
                let final_size = get_file_size_kb(output);
                logger::nerd_output_summary(input, output, original_size, final_size, "pngquant + Grayscale", total_time);
            }
            return Ok(result_with_time("pngquant + Grayscale", start));
        }
    }

    // Branch B: Grayscale Fails OR User Rejected
    let mut resize_input = &oxi_out;

    if gray_size < oxi_size {
        // Finish progress bar before showing prompts
        if let Some(ref mut bar) = progress {
            bar.set(50);
            bar.finish();
        }
        progress = None; // Clear progress bar reference
        // Grayscale is smaller, offer it as base for resizing
        let should_use_grayscale = if auto_yes {
            if nerd { println!("   [Auto-yes enabled, using grayscale for resizing]"); }
            true
        } else {
            Confirm::new().with_prompt("Target unreachable in Color. Proceed with Grayscale Resizing?").default(true).interact()?
        };
        if should_use_grayscale {
            resize_input = &gray_out;
        } else {
            // User rejected grayscale - ask if they want to resize color instead
            let should_resize_color = if auto_yes {
                if nerd { println!("   [Auto-yes enabled, resizing color image]"); }
                true
            } else {
                Confirm::new().with_prompt("Resize the Color image instead?").default(false).interact()?
            };
            if !should_resize_color {
                // User rejected all options - save best effort and exit
                if let Some(ref p) = _color_candidate_path {
                    fs::copy(p, output)?;
                    fs::remove_file(p).ok();
                } else {
                    fs::copy(&oxi_out, output)?;
                }
                fs::remove_file(&oxi_out).ok();
                fs::remove_file(&gray_out).ok();
                if let Some(ref mut bar) = progress {
                    bar.set(100);
                    bar.finish();
                }
                if nerd {
                    let total_time = start.elapsed().as_secs_f64();
                    let final_size = get_file_size_kb(output);
                    logger::nerd_output_summary(input, output, original_size, final_size, "pngquant (Best Effort Color)", total_time);
                }
                println!("   Keeping best color version ({} KB).", get_file_size_kb(output));
                return Ok(result_with_time("pngquant (Best Effort Color)", start));
            }
            // else: proceed with color resize
        }
    } else {
        // Finish progress bar before showing prompts
        if let Some(ref mut bar) = progress {
            bar.set(50);
            bar.finish();
        }
        progress = None; // Clear progress bar reference
        // Gray is not smaller than oxi - ask about resizing color
        let should_resize = if auto_yes {
            if nerd { println!("   [Auto-yes enabled, resizing image]"); }
            true
        } else {
            Confirm::new().with_prompt("Target unreachable. Resize image dimensions?").default(false).interact()?
        };
        if !should_resize {
            // Save best effort
            if let Some(ref p) = _color_candidate_path {
                fs::copy(p, output)?;
                fs::remove_file(p).ok();
            } else {
                fs::copy(&oxi_out, output)?;
            }
            fs::remove_file(&oxi_out).ok();
            fs::remove_file(&gray_out).ok();
            if let Some(ref mut bar) = progress {
                bar.set(100);
                bar.finish();
            }
            if nerd {
                let total_time = start.elapsed().as_secs_f64();
                let final_size = get_file_size_kb(output);
                logger::nerd_output_summary(input, output, original_size, final_size, "pngquant (Best Effort)", total_time);
            }
            println!("   Keeping best version ({} KB).", get_file_size_kb(output));
            return Ok(result_with_time("pngquant (Best Effort)", start));
        }
    }

    // 4. RESIZE LOOP
    if nerd {
        logger::nerd_stage(4, "Image Resizing");
        logger::nerd_result("Tool", "magick", false);
        logger::nerd_result("Strategy", "Resizing image dimentions using Binary search as Scale index(too lossy)", false);
        logger::nerd_result("Complexity", "O(log n)", false);
        logger::nerd_cmd("magick <in> -resize <scale>% <out>");
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
            let action = if size <= target { "min=mid+1" } else { "max=mid-1" };
            if nerd {
                logger::nerd_scale_attempt(attempts, 8, mid_scale as u8, size, target, elapsed_ms, action);
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
        let should_save_smallest = if auto_yes {
            if nerd { println!("   [Auto-yes enabled, saving smallest possible]"); }
            true
        } else {
            Confirm::new().with_prompt("Target unreachable. Save smallest possible?").default(true).interact()?
        };
        if should_save_smallest {
            final_size = get_file_size_kb(&resize_out);
            fs::copy(&resize_out, output)?;
        }
    }
    // Cleanup
    fs::remove_file(&oxi_out).ok();
    fs::remove_file(&gray_out).ok();
    fs::remove_file(&resize_out).ok();
    if let Some(ref p) = _color_candidate_path { fs::remove_file(p).ok(); }
    if nerd {
        let total_time = start.elapsed().as_secs_f64();
        logger::nerd_output_summary(input, output, original_size, final_size, "PNG Hybrid Chain", total_time);
    }
    Ok(result_with_time("Hybrid Chain", start))
}

// PDF: Binary Search (Optimal) with Floor Detection
fn compress_pdf(input: &str, output: &str, target_kb: Option<u64>, _level: Option<CompressionLevel>, nerd: bool, auto_yes: bool) -> Result<CompResult> {
    let total_start = Instant::now();
    let original_size = get_file_size_kb(input);
    let mut _gs_calls: u32 = 0;
    if let Some(target) = target_kb {
        if target >= original_size {
            println!("Requested size ({}) KB is larger than or equal to original file size ({} KB). No compression performed.", target, original_size);
            let should_keep = if auto_yes {
                if nerd { println!("   [Auto-yes enabled, keeping original]"); }
                true
            } else {
                Confirm::new().with_prompt("Keep original file?").default(true).interact()?
            };
            if should_keep {
                fs::copy(input, output)?;
                return Ok(result_with_time("No compression (requested size >= original)", total_start));
            } else {
                return Err(anyhow!("Compression cancelled by user."));
            }
        }
    }

    if target_kb.is_none() {
        // Smart preset selection based on file size
        let preset = if original_size > 50_000 {
            // Large files (>50MB): aggressive compression
            "/ebook"
        } else if original_size > 10_000 {
            // Medium files (10-50MB): balanced compression
            "/ebook"
        } else if original_size > 1_000 {
            // Small-medium files (1-10MB): moderate compression
            "/printer"
        } else {
            // Small files (<1MB): light compression
            "/printer"
        };
        
        if nerd {
            logger::nerd_stage(1, "Smart Compression");
            logger::nerd_result("Tool", "Ghostscript", false);
            logger::nerd_result("Strategy", &format!("Preset-based compression ({})", preset), false);
            logger::nerd_result("Reason", &format!("Selected {} for {} KB file", preset, original_size), false);
        }
        let progress = PacmanProgress::new(1, "Eating those bytes...");
        run_gs(input, output, preset, None)?;
        progress.finish();
        if nerd {
            let total_time = total_start.elapsed().as_secs_f64();
            let final_size = get_file_size_kb(output);
            logger::nerd_output_summary(input, output, original_size, final_size, &format!("Smart Compression ({})", preset), total_time);
        }
        return Ok(result_with_time(format!("Smart Compression ({})", preset), total_start));
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
        let should_save_floor = if auto_yes {
            if nerd { println!("   [Auto-yes enabled, saving smallest possible version]"); }
            true
        } else {
            Confirm::new().with_prompt("   Save the smallest possible version?").default(true).interact()?
        };
        if !should_save_floor {
            let _ = fs::remove_file(&temp_output);
            return Err(anyhow!("Compression cancelled."));
        }
        fs::rename(&temp_output, output)?;
        if nerd {
            let total_time = total_start.elapsed().as_secs_f64();
            let final_size = get_file_size_kb(output);
            logger::nerd_output_summary(input, output, original_size, final_size, "Floor (Min Quality)", total_time);
        }
        println!("Tip: Could not reach target size without destroying quality.\n   Try a higher size.");
        return Ok(result_with_time("Floor (Min Quality)", total_start));
    }
    
    // Smart DPI range based on compression ratio
    let compression_ratio = original_size as f64 / target as f64;
    let (mut min_dpi, mut max_dpi): (u64, u64) = match compression_ratio {
        r if r > 10.0 => (50, 150),   // Extreme compression
        r if r > 3.0  => (72, 250),   // Heavy compression
        r if r > 2.0  => (100, 400),  // Moderate compression
        _             => (150, 600),  // Light compression
    };
    
    if nerd {
        logger::nerd_stage(2, "Size Reduction");
        logger::nerd_result("Tool", "Ghostscript", false);
        logger::nerd_result("Strategy", "PDF compression using Binary search with adaptive DPI range", false);
        logger::nerd_result("Complexity", "O(log n) search iterations, O(n) compression per attempt", false);
        logger::nerd_cmd("gs ... -dColorImageResolution=<dpi> ...");
        logger::nerd_result(
            "Smart DPI Range", 
            &format!("{}-{} DPI (ratio: {:.1}:1)", min_dpi, max_dpi, compression_ratio),
            false
        );
        logger::nerd_result("Note", "Each iteration re-renders entire PDF (3-6s per attempt is normal)", false);
    }
    let mut best_dpi: u64 = 0;
    let mut best_size: u64 = 0;
    let mut found_valid = false;
    let max_iterations: u32 = 14;
    let mut attempts: u32 = 0;
    let mut search_progress = PacmanProgress::new(14, "Eating those bytes...");
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
                logger::nerd_attempt(attempts, 14, mid_dpi, size, target, iter_start.elapsed().as_millis(), action_str);
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
            println!();
            println!("  {} Target achieved at {} DPI ({} KB)", "└─".cyan(), best_dpi.to_string().green(), best_size.to_string().green());
            println!("     Compressing PDF at {} DPI to final output...", best_dpi.to_string().cyan());
            println!();
            let total_time = total_start.elapsed().as_secs_f64();
            logger::nerd_output_summary(input, output, original_size, best_size, &format!("Ghostscript Binary Search ({} DPI)", best_dpi), total_time);
        } else if best_dpi < 50 {
            println!("\n{}", "   Note: Very low DPI - images may appear pixelated.".yellow());
        }
        Ok(result_with_time(format!("Binary Search ({} DPI)", best_dpi), total_start))
    } else {
        run_gs(input, output, "/screen", None)?;
        Ok(result_with_time("Fallback /screen", total_start))
    }
}

// ==================== SHARED FALLBACK LOGIC ====================

fn handle_fallback_options(output: &str, target: u64, current_size: u64, nerd: bool, format: &str) -> Result<CompResult> {
    let fallback_start = Instant::now();
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
                return Ok(result_with_time(format!("{} + Grayscale", format), fallback_start));
            } else if nerd { logger::nerd_result("Grayscale size", &format!("{} KB (Still > Target)", gray_size), true); }
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
            return Ok(result_with_time(format!("{} + Resize {}%", format, best_scale), fallback_start));
        }
    }

    println!("   Keeping the {} KB version.", get_file_size_kb(output));
    Ok(result_with_time("Best Effort", fallback_start))
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