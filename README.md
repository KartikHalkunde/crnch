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



**crnch** is a Rust CLI tool that compresses PNG, JPG, and PDF files to the smallest possible size without visible quality loss. It wraps industry-standard tools (`ghostscript`, `pngquant`, `imagemagick`, `jpegoptim`, `oxipng`) into a unified interface.

![Rust](https://img.shields.io/badge/Made%20with-Rust-orange)
![Arch](https://img.shields.io/badge/Arch-Native-blue)

## 🚀 Features

- **Multi-stage Compression:** Uses waterfall logic for PNG/JPG (lossless, quantization, grayscale, resize) and binary search for PDF DPI.
- **Target Size:** Specify exact output size (e.g., `--size 200k`).
- **Nerd Mode:** Detailed technical output with color-coded stages and tool info.
- **Single Pacman Progress Bar:** Smooth, animated progress for all operations.
- **Dependency Detection:** Checks for required tools and provides install tips.
- **Unified Output:** All results saved as `crnched_<filename>.<ext>`.
- **Interactive Prompts:** Offers grayscale/resize options if target size is unreachable.

## 🖼️ Supported Formats & Tools

- **JPG:** `jpegoptim` (lossless), `imagemagick` (lossy, extent targeting)
- **PNG:** `oxipng` (lossless), `pngquant` (quantization), `imagemagick` (grayscale/resize)
- **PDF:** `ghostscript` (standard preset, binary search for DPI)

## 💡 Usage

```bash
crnch <file> [--size <target>] [--level <low|medium|high>] [--output <path>] [--nerd]
```

### Example

```bash
crnch image.png --size 200k
crnch document.pdf --nerd
```

## 📦 Installation

**Arch Linux:**
```bash
sudo pacman -S ghostscript imagemagick pngquant jpegoptim oxipng
```