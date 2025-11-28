# crnch ⚡

**crnch** is a blazing fast, Arch-native CLI wrapper that squeezes PDFs, PNGs, and JPGs to the absolute limit without losing visual quality. 

It wraps industry-standard engines (`ghostscript`, `pngquant`, `imagemagick`) into a single, unified interface.

![Rust](https://img.shields.io/badge/Made%20with-Rust-orange)
![Arch](https://img.shields.io/badge/Arch-Native-blue)

## 🚀 Features

- **Smart Wrapper:** Uses the best engine for the job.
  - JPG → `ImageMagick` (via `jpeg:extent`)
  - PNG → `pngquant` (Lossy quantization)
  - PDF → `Ghostscript` (Hybrid presets)
- **Target Size:** Specify exactly how big you want the file (e.g., `-s 200k`).
- **Interactive Menu:** Just run `crnch file.pdf` and pick a level.
- **Dependency Check:** Automatically detects missing tools and tells you how to install them.
- **Unified Branding:** Outputs files as `crnched_<filename>.<ext>`.

## 📦 Installation

### Prerequisites
`crnch` relies on external binaries. 

**Arch Linux:**
```bash
sudo pacman -S ghostscript imagemagick pngquant
# OR via Yay
yay -S ghostscript imagemagick pngquant