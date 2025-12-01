mod checks;
mod compression;
mod logger;
mod utils;

use clap::Parser;
use regex::Regex;
use std::path::Path;
use compression::CompressionLevel;

#[derive(Parser)]
#[command(name = "crnch")]
#[command(about = "Squeeze your files. Fast.", version)]
#[command(author = "Kartik <kartikhalkunde26@gmail.com>")]
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

    /// Verbosity level (-v=verbose, -vv=nerd mode)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    verbose: u8,

    /// Enable nerd mode (detailed technical output, same as -vv)
    #[arg(long)]
    nerd: bool,

    /// Assume yes to all prompts (non-interactive mode)
    #[arg(short = 'y', long)]
    yes: bool,
}

fn main() {
    // 1. Check Dependencies (Cross-Distro)
    if let Err(e) = checks::check_dependencies() {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    let cli = Cli::parse();

    // Set verbosity level: --nerd = 3, -vv = 3, -v = 2, default = 1
    let verbosity = if cli.nerd { 3 } else { cli.verbose.saturating_add(1).min(3) };
    logger::set_verbosity(verbosity);
    let is_nerd = verbosity >= 3;

    // 2. If no size/level provided, just do lossless compression (oxipng) without prompt

    // 3. Determine Output Filename
    let input_path = Path::new(&cli.file);
    
    if !input_path.exists() {
        logger::log_error(&format!("File '{}' not found.", cli.file));
        std::process::exit(1);
    }

    let output_path = match cli.output {
        Some(ref p) => {
            if p.starts_with("/etc/") || p.starts_with("/sys/") {
                logger::log_error("Cannot write to system directories");
                std::process::exit(1);
            }
            if Path::new(p).exists() {
                if !dialoguer::Confirm::new()
                    .with_prompt(format!("Overwrite {}?", p))
                    .default(false)
                    .interact()
                    .unwrap_or(false) {
                    std::process::exit(0);
                }
            }
            p.clone()
        },
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
    let target_kb: Option<u64> = cli.size.as_ref().and_then(|s| utils::parse_size(s));

    // Start logging
    if is_nerd {
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
    match compression::compress_file(&cli.file, &output_path, size_option.clone(), level_option, is_nerd, cli.yes) {
        Ok(result) => {
            if let Ok(meta_new) = std::fs::metadata(&output_path) {
                let new_kb = meta_new.len() / 1024;
                
                if !is_nerd {
                    logger::log_done();
                    
                    // Use enhanced summary with timing in verbose mode
                    if verbosity >= 2 {
                        logger::log_summary(
                            &cli.file, 
                            &output_path, 
                            input_size_kb, 
                            new_kb, 
                            Some(&result.algorithm),
                            Some(result.time_ms)
                        );
                    } else {
                        logger::log_result(&cli.file, &output_path, input_size_kb, new_kb);
                    }
                    
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
        Err(e) => {
            logger::log_error(&format!("Failed: {}", e));
            std::process::exit(1);
        }
    }
}