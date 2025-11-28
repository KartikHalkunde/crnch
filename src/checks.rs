use anyhow::Result;
use colored::*;
use which::which;
use os_info;

pub fn check_dependencies() -> Result<()> {
    let tools = ["gs", "magick", "pngquant"];
    let mut missing_tools = Vec::new();

    // 1. Check for binaries
    for tool in tools {
        if which(tool).is_err() {
            missing_tools.push(tool);
        }
    }

    if missing_tools.is_empty() {
        return Ok(());
    }

    // 2. If missing, report error and give specific install instructions
    println!("\n{} Missing dependencies: {:?}", "❌ Error:".red().bold(), missing_tools);
    println!("{}", "crnch relies on external industry-standard tools.".yellow());
    println!("\n{}", "⬇️  Run this command to install them:".blue().bold());

    let info = os_info::get();
    
    // Smart Distro Detection
    match info.os_type() {
        os_info::Type::Arch => {
            println!("   {}", "sudo pacman -S ghostscript imagemagick pngquant".green());
            println!("   {} {}", "OR via Yay:".dimmed(), "yay -S ghostscript imagemagick pngquant".green());
        },
        os_info::Type::Ubuntu | os_info::Type::Debian | os_info::Type::Pop | os_info::Type::Mint => {
            println!("   {}", "sudo apt update && sudo apt install ghostscript imagemagick pngquant".green());
        },
        os_info::Type::Fedora | os_info::Type::CentOS => {
            println!("   {}", "sudo dnf install ghostscript ImageMagick pngquant".green());
        },
        os_info::Type::Macos => {
            println!("   {}", "brew install ghostscript imagemagick pngquant".green());
        },
        _ => {
            // Fallback / Unknown Linux
            println!("   {}", "Arch:   sudo pacman -S ghostscript imagemagick pngquant".green());
            println!("   {}", "Debian: sudo apt install ghostscript imagemagick pngquant".green());
            println!("   {}", "Mac:    brew install ghostscript imagemagick pngquant".green());
        }
    }

    println!();
    std::process::exit(1);
}