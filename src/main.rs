mod checks;
mod compression;
mod logger;
mod utils;

use clap::Parser;
use std::path::Path;
use compression::CompressionLevel;

#[derive(Parser)]
#[command(name = "crnch")]
#[command(about = "Intelligent file compression for PNG, JPG, and PDF", long_about = None)]
#[command(version)]
#[command(author = "Kartik <kartikhalkunde26@gmail.com>")]
#[command(override_usage = "crnch <FILE> [OPTIONS]")]
#[command(after_help = "EXAMPLES:\n  crnch image.png                      Auto-compress PNG (lossless optimization)\n  crnch document.pdf                   Auto-compress PDF (standard compression)\n  crnch photo.jpg --size 200k          Compress JPG to exactly 200KB\n  crnch file.png --size 1.5m --nerd    Compress to 1.5MB with detailed output\n  crnch file.png --output result.png   Compress with custom output path\n  crnch image.png -y                   Auto-compress without prompts\n\nNOTE:\n  All options are optional! Just 'crnch file.png' works perfectly.\n  --size is only needed if you want a specific target file size.\n\nSUPPORTED FORMATS:\n  .jpg, .jpeg    JPEG images\n  .png           PNG images\n  .pdf           PDF documents\n\nSIZE FORMAT (optional):\n  Examples: 200k, 1.5m, 500kb, 2mb, 1g, 1.5gb\n  Units: k/kb (kilobytes), m/mb (megabytes), g/gb (gigabytes)\n\nFor more information, visit: https://github.com/KartikHalkunde/crnch")]
struct Cli {
    /// The file to compress
    file: String,

    /// Target size (e.g., '200k', '1.5m') - Optional, auto-compress if not specified
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

    // 2. Validate input file exists
    let input_path = Path::new(&cli.file);
    
    if !input_path.exists() {
        logger::log_error(&format!("File '{}' not found.", cli.file));
        eprintln!("\nTip: Check the file path and try again.");
        eprintln!("     Use absolute path or relative path from current directory.");
        std::process::exit(1);
    }
    
    // 3. Validate file is not a directory
    if input_path.is_dir() {
        logger::log_error(&format!("'{}' is a directory, not a file.", cli.file));
        eprintln!("\nTip: Compress individual files, not directories.");
        std::process::exit(1);
    }
    
    // 4. Validate file extension
    if let Err(e) = utils::validate_file_extension(&cli.file) {
        logger::log_error(&e.to_string());
        std::process::exit(1);
    }
    
    // 5. Validate file is readable
    if let Err(e) = std::fs::File::open(&cli.file) {
        logger::log_error(&format!("Cannot read file '{}': {}", cli.file, e));
        eprintln!("\nTip: Check file permissions with: ls -l {}", cli.file);
        std::process::exit(1);
    }
    
    // 6. Validate size parameter if provided
    if let Some(ref size_str) = cli.size {
        if let Err(e) = utils::validate_size(size_str) {
            logger::log_error(&e.to_string());
            std::process::exit(1);
        }
    }

    // 7. Determine and validate output path
    let output_path = match cli.output {
        Some(ref p) => {
            // Validate output path
            if let Err(e) = utils::validate_output_path(p) {
                logger::log_error(&e.to_string());
                std::process::exit(1);
            }
            
            // Check if output file already exists
            if Path::new(p).exists() {
                if cli.yes {
                    // Auto-yes mode: skip overwrite
                    logger::log_warning(&format!("File '{}' already exists. Skipping (auto-yes mode).", p));
                    std::process::exit(0);
                }
                
                match dialoguer::Confirm::new()
                    .with_prompt(format!("Overwrite {}?", p))
                    .default(false)
                    .interact() {
                    Ok(true) => {},
                    Ok(false) => {
                        println!("Operation cancelled.");
                        std::process::exit(0);
                    },
                    Err(e) => {
                        logger::log_error(&format!("Input error: {}", e));
                        std::process::exit(1);
                    }
                }
            }
            p.clone()
        },
        None => {
            let stem = input_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            let ext = input_path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin")
                .to_lowercase();
            format!("crnched_{}.{}", stem, ext)
        }
    };
    
    // 8. Check if input and output are the same file
    if input_path.canonicalize().ok() == Path::new(&output_path).canonicalize().ok() {
        logger::log_error("Input and output files cannot be the same.");
        eprintln!("\nTip: Use --output to specify a different output file.");
        std::process::exit(1);
    }

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

    // 9. Run Compression
    match compression::compress_file(&cli.file, &output_path, size_option.clone(), level_option, is_nerd, cli.yes) {
        Ok(result) => {
            // Verify output file was created
            if !Path::new(&output_path).exists() {
                logger::log_error("Compression completed but output file not found.");
                eprintln!("\nThis may indicate a system error. Check disk space and permissions.");
                std::process::exit(1);
            }
            
            match std::fs::metadata(&output_path) {
                Ok(meta_new) => {
                    let new_kb = meta_new.len() / 1024;
                    
                    // Sanity check: output file should not be empty
                    if new_kb == 0 {
                        logger::log_error("Output file is empty (0 bytes).");
                        eprintln!("\nThis indicates a compression failure. The original file is intact.");
                        let _ = std::fs::remove_file(&output_path);
                        std::process::exit(1);
                    }
                    
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
                        
                        // Validation check - only show warning if target was significantly missed
                        if let Some(target_str) = size_option.as_ref() {
                            if let Some(target_val) = utils::parse_size(target_str) {
                                // Only warn if we're more than 20% over target (not just 10%)
                                if new_kb > target_val + (target_val / 5) {
                                    // Get file extension to provide relevant suggestions
                                    let ext = input_path.extension()
                                        .and_then(|e| e.to_str())
                                        .unwrap_or("")
                                        .to_lowercase();
                                    
                                    logger::log_warning("Could not reach target size.");
                                    match ext.as_str() {
                                        "pdf" => {
                                            println!("   Tip: Try a larger target size, or use lower quality settings.");
                                        },
                                        "jpg" | "jpeg" => {
                                            println!("   Tip: Try resizing the image dimensions for better compression.");
                                        },
                                        "png" => {
                                            println!("   Tip: Try resizing the image or converting to JPEG format.");
                                        },
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                },
                Err(e) => {
                    logger::log_error(&format!("Cannot read output file: {}", e));
                    std::process::exit(1);
                }
            }
        },
        Err(e) => {
            let error_msg = e.to_string();
            logger::log_error(&format!("Compression failed: {}", error_msg));
            
            // Provide helpful tips based on error type
            if error_msg.contains("No such file") || error_msg.contains("not found") {
                eprintln!("\nTip: Check that all required tools are installed.");
                eprintln!("     Run: crnch --help for installation instructions.");
            } else if error_msg.contains("Permission denied") {
                eprintln!("\nTip: Check file and directory permissions.");
            } else if error_msg.contains("Disk quota") || error_msg.contains("No space") {
                eprintln!("\nTip: Free up disk space and try again.");
            }
            
            std::process::exit(1);
        }
    }
}