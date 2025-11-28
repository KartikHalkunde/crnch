mod checks;
mod compression;
mod logger;

use clap::Parser;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Select};
use regex::Regex;
use std::path::Path;
use compression::CompressionLevel;

#[derive(Parser)]
#[command(name = "crnch")]
#[command(about = "Squeeze your files. Fast.", version = "1.0")]
#[command(author = "Kartik <innotelesoft.com>")]
struct Cli {
    /// The file to compress
    file: String,

    /// Target size (e.g., '200k', '1.5m')
    #[arg(short, long)]
    size: Option<String>,

    /// Compression level (overrides size)
    #[arg(short, long, value_enum)]
    level: Option<CompressionLevel>,

    /// Custom output path
    #[arg(short, long)]
    output: Option<String>,

    /// Enable nerd mode (detailed technical output)
    #[arg(long, visible_alias = "verbose", short = 'v')]
    nerd: bool,
}

fn main() {
    // 1. Check Dependencies (Cross-Distro)
    if let Err(e) = checks::check_dependencies() {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    let mut cli = Cli::parse();

    // Set nerd mode globally
    logger::set_nerd_mode(cli.nerd);

    // 2. Interactive Mode (If no size/level provided)
    if cli.size.is_none() && cli.level.is_none() {
        let selections = &[
            "Low Compression (Better Quality)",
            "Medium Compression (Balanced)",
            "High Compression (Smallest Size)",
        ];

        println!("{}", "No options provided. Select compression mode:".yellow());
        
        let selection = Select::with_theme(&ColorfulTheme::default())
            .default(1)
            .items(&selections[..])
            .interact()
            .unwrap();

        cli.level = Some(match selection {
            0 => CompressionLevel::Low,
            1 => CompressionLevel::Medium,
            _ => CompressionLevel::High,
        });
    }

    // 3. Determine Output Filename
    let input_path = Path::new(&cli.file);
    
    if !input_path.exists() {
        logger::log_error(&format!("File '{}' not found.", cli.file));
        std::process::exit(1);
    }

    let output_path = match cli.output {
        Some(p) => p,
        None => {
            let stem = input_path.file_stem().unwrap().to_str().unwrap();
            let ext = input_path.extension().unwrap().to_str().unwrap().to_lowercase();
            format!("crnched_{}.{}", stem, ext)
        }
    };

    // Get input size for logging
    let input_size_kb = std::fs::metadata(&cli.file)
        .map(|m| m.len() / 1024)
        .unwrap_or(0);

    // Parse target for nerd mode header
    let target_kb: Option<u64> = cli.size.as_ref().and_then(|s| {
        let re = Regex::new(r"(?i)^(\d+(?:\.\d+)?)(k|m|kb|mb)?$").ok()?;
        let caps = re.captures(s)?;
        let val: f64 = caps[1].parse().ok()?;
        let unit = caps.get(2).map_or("k", |m| m.as_str()).to_lowercase();
        match unit.as_str() {
            "m" | "mb" => Some((val * 1024.0) as u64),
            _ => Some(val as u64),
        }
    });

    // Start logging
    if cli.nerd {
        logger::nerd_header();
        logger::nerd_file_info(&cli.file, input_size_kb, target_kb);
    } else {
        logger::log_start(&cli.file);
        if let Some(target) = &cli.size {
            logger::log_target(target);
        } else if let Some(lvl) = &cli.level {
            println!("   Level: {:?}", lvl);
        }
    }

    let size_option = cli.size.clone();
    let level_option = cli.level;

    // 4. Run Compression
    match compression::compress_file(&cli.file, &output_path, size_option.clone(), level_option, cli.nerd) {
        Ok(_) => {
            if let Ok(meta_new) = std::fs::metadata(&output_path) {
                let new_kb = meta_new.len() / 1024;
                
                if !cli.nerd {
                    logger::log_done();
                    logger::log_result(&cli.file, &output_path, input_size_kb, new_kb);
                    
                    // Validation check
                    if let Some(target_str) = size_option.as_ref() {
                        let re = Regex::new(r"(\d+)").unwrap();
                        if let Some(caps) = re.captures(target_str) {
                            let target_val: u64 = caps[1].parse().unwrap_or(0);
                            if new_kb > target_val + (target_val / 10) {
                                logger::log_warning("Could not reach target size without destroying quality.");
                                println!("   Try resizing the image dimensions first.");
                            }
                        }
                    }
                }
            }
        },
        Err(e) => logger::log_error(&format!("Failed: {}", e)),
    }
}