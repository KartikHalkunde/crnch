```bash
                                        █████                                   ███                ██████          
                                       ░░███                                   ░░░                ███░░███         
  ██████  ████████  ████████    ██████  ░███████                               ████  ████████    ░███ ░░░   ██████ 
 ███░░███░░███░░███░░███░░███  ███░░███ ░███░░███        ██████████ ██████████░░███ ░░███░░███  ███████    ███░░███
░███ ░░░  ░███ ░░░  ░███ ░███ ░███ ░░░  ░███ ░███       ░░░░░░░░░░ ░░░░░░░░░░  ░███  ░███ ░███ ░░░███░    ░███ ░███
░███  ███ ░███      ░███ ░███ ░███  ███ ░███ ░███                              ░███  ░███ ░███   ░███     ░███ ░███
░░██████  █████     ████ █████░░██████  ████ █████                             █████ ████ █████  █████    ░░██████ 
 ░░░░░░  ░░░░░     ░░░░ ░░░░░  ░░░░░░  ░░░░ ░░░░░                             ░░░░░ ░░░░ ░░░░░  ░░░░░      ░░░░░░  
                                                                                                                   
````



**crnch** is a blazing-fast Rust CLI tool that intelligently compresses PNG, JPG, and PDF files to target sizes with minimal quality loss. It orchestrates industry-standard tools (`ghostscript`, `pngquant`, `imagemagick`, `jpegoptim`, `oxipng`) through sophisticated multi-stage algorithms and binary search optimization.

![Rust](https://img.shields.io/badge/Made%20with-Rust-orange)
![Arch](https://img.shields.io/badge/Arch-Native-blue)
![License](https://img.shields.io/badge/License-MIT-green)

## ✨ Features

- **🎯 Intelligent Target Sizing:** Precisely hit file size targets using binary search algorithms (e.g., `--size 200k`)
- **🔬 Nerd Mode (`--nerd`):** Professional box-formatted output with:
  - System information (OS, CPU, RAM, tool versions)
  - Image dimensions and metadata
  - Algorithm complexity analysis
  - Stage-by-stage processing details
  - Comprehensive compression statistics (ratio, reduction %, time)
- **🌊 Multi-Stage Waterfall Logic:**
  - **PNG:** Lossless (oxipng) → Quantization (pngquant) → Hybrid Binary Search → Grayscale → Resize
  - **JPG:** Lossless (jpegoptim) → Lossy + ImageMagick resize/quality tuning
  - **PDF:** Standard compression → Binary search DPI optimization (1-2400 range)
- **📊 Pacman Progress Bar:** Smooth, animated progress with real-time updates
- **🔍 Smart Dependency Detection:** Auto-checks for required tools and provides installation guidance
- **🎨 Color-Coded Output:** Beautiful terminal UI with hierarchical formatting
- **💬 Interactive Prompts:** Offers fallback options (grayscale/resize) when targets are unreachable
- **⚡ Optimized Performance:** Release builds with aggressive optimizations

## 🛠️ Supported Formats & Tools

| Format | Tools | Strategies |
|--------|-------|------------|
| **JPG** | `jpegoptim`, `imagemagick` | Lossless optimization → Quality reduction → Resize with extent |
| **PNG** | `oxipng`, `pngquant`, `imagemagick` | Lossless → 256-color quantization → Grayscale → Dimension resize |
| **PDF** | `ghostscript` | Standard presets (`/printer`) → Binary search DPI (O(log n) iterations) |

## 🚀 Usage

```bash
crnch <file> [OPTIONS]

OPTIONS:
    --size <SIZE>        Target file size (e.g., 200k, 2m, 1.5mb)
    --level <LEVEL>      Compression level: low, medium, high [default: medium]
    --output <PATH>      Custom output path [default: crnched_<filename>]
    --nerd, -vvv         Enable detailed nerd mode with technical insights
    --auto-yes, -y       Skip interactive prompts (accept all defaults)
```

### Examples

```bash
# Compress PNG to exactly 200KB with nerd mode
crnch image.png --size 200k --nerd

# Compress PDF with automatic optimization
crnch document.pdf

# High compression with custom output
crnch photo.jpg --level high --output compressed.jpg

# Batch processing with target size
for file in *.png; do crnch "$file" --size 500k --auto-yes; done
```

## 📦 Installation

### Arch Linux
```bash
# Install dependencies
sudo pacman -S ghostscript imagemagick pngquant jpegoptim oxipng

# Build from source
git clone https://github.com/KartikHalkunde/crnch.git
cd crnch
cargo build --release

# Binary will be at: target/release/crnch
sudo cp target/release/crnch /usr/local/bin/
```

### Other Linux Distributions
```bash
# Debian/Ubuntu
sudo apt install ghostscript imagemagick pngquant jpegoptim oxipng

# Fedora
sudo dnf install ghostscript ImageMagick pngquant jpegoptim oxipng
```

## 🎨 Nerd Mode Output

```
╔═══════════════════════════════════════════════════════════════════════╗
║                          SYSTEM INFORMATION                           ║
╠═══════════════════════════════════════════════════════════════════════╣
  OS: Arch Linux                Arch: x86_64
  CPU: 12th Gen Intel(R) Core(TM) i3-1215U
  RAM: 7.4 GB
╠═══════════════════════════════════════════════════════════════════════╣
  Ghostscript: 10.06.0
  ImageMagick: 7.1.2-8 Q16-HDRI
  pngquant:    3.0.3
╚═══════════════════════════════════════════════════════════════════════╝

╔═══════════════════════════════════════════════════════════════════════╗
║                            INPUT FILE                                 ║
╠═══════════════════════════════════════════════════════════════════════╣
  Filename: image.png
  Type:     PNG
  Path:     /path/to/image.png
  Size:     2.45 MB (2569874 bytes)
  ├─ Dimensions: 1920 × 1080 pixels
  └─ Aspect Ratio: 16:9
╠═══════════════════════════════════════════════════════════════════════╣
  Target:   500 KB
╚═══════════════════════════════════════════════════════════════════════╝

───────────────────────────────────────────────────────────────────────────
[STAGE 1] Lossless Compression
───────────────────────────────────────────────────────────────────────────
   |- Algorithm: Oxipng
   |- Complexity: O(n) CPU-bound
   |- Strategy: PNG optimization (zlib compression level 6)

╔═══════════════════════════════════════════════════════════════════════╗
║                         COMPRESSION RESULT                            ║
╠═══════════════════════════════════════════════════════════════════════╣
  Output File: crnched_image.png
  Method:      Hybrid (Oxipng + Binary Search)
╠═══════════════════════════════════════════════════════════════════════╣
  Size:        2.45 MB → 498 KB
  Reduction:   79.7% (2014 KB saved)
  Ratio:       5.04:1
  Time:        3.42s
╚═══════════════════════════════════════════════════════════════════════╝
```

## 🧠 Algorithm Details

### PNG Compression Strategy
1. **Lossless (oxipng):** Optimize PNG structure without quality loss
2. **Quantization (pngquant):** Reduce to 256 colors if target not met
3. **Binary Search:** Fine-tune quality parameter (0-100) using bisection
4. **Grayscale Fallback:** Convert to B&W if color quantization insufficient
5. **Resize:** Reduce dimensions as last resort (maintains aspect ratio)

### PDF Compression Strategy
1. **Standard Compression:** Apply Ghostscript `/printer` preset
2. **Binary Search DPI:** Optimize DPI (1-2400 range) in O(log n) iterations
3. **Floor Detection:** Calculate minimum achievable size with `/screen` preset

### JPG Compression Strategy
1. **Lossless (jpegoptim):** Strip metadata, optimize Huffman tables
2. **Quality Reduction:** Binary search quality parameter (1-100)
3. **Resize + Extent:** Use ImageMagick to resize and pad to exact target

## 📊 Performance

- **Compression Speed:** ~1-5s for typical images (1-5 MB)
- **PDF Binary Search:** 10-14 iterations to converge on target
- **Memory Efficient:** Streams data, minimal RAM overhead

## 🤝 Contributing

Contributions welcome! Please ensure:
- Code compiles with `cargo build --release` (zero warnings)
- All compression paths tested
- Nerd mode output sections included for new features

## 📄 License

MIT License - See LICENSE file for details

## 🙏 Credits

Built with ❤️ using:
- [Ghostscript](https://www.ghostscript.com/) - PDF compression
- [ImageMagick](https://imagemagick.org/) - Image manipulation
- [pngquant](https://pngquant.org/) - PNG quantization
- [jpegoptim](https://github.com/tjko/jpegoptim) - JPEG optimization
- [oxipng](https://github.com/shssoichiro/oxipng) - PNG optimization

---

**Made with Rust 🦀 | Optimized for Arch Linux 🐧**