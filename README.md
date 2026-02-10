# nitrohydra

A multi-monitor wallpaper picker.

> [!WARNING]
> **Only supports Linux Cinnamon (X11).**

Browse a folder of images, select one wallpaper per monitor, and apply them instantly. Nitrohydra composes a single spanned image behind the scenes, so each monitor gets its own wallpaper even though Cinnamon only supports a single background URI.

## Features

- Thumbnail gallery with adjustable size
- Parallel image loading with persistent disk cache
- Per-monitor wallpaper assignment (select #1 for left, #2 for right)
- Cover-resize: images are scaled and center-cropped to fill each monitor without letterboxing

## Building from source

Requires Rust 2024 edition (1.85+) and `xrandr` / `gsettings` available on PATH.

```bash
git clone https://github.com/alexamy/nitrohydra.git
cd nitrohydra
cargo build --release
./target/release/nitrohydra
```

## Built with ❤️

This project was built by a human and [Claude Code](https://claude.com/claude-code) working together.
