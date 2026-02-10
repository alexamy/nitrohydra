# nitrohydra

A multi-monitor wallpaper picker for Linux Cinnamon (X11).

Browse a folder of images, select one wallpaper per monitor, and apply them instantly. Nitrohydra composes a single spanned image behind the scenes, so each monitor gets its own wallpaper even though Cinnamon only supports a single background URI.

## Features

- Thumbnail gallery with adjustable size
- Parallel image loading with persistent disk cache
- Per-monitor wallpaper assignment (select #1 for left, #2 for right)
- Cover-resize: images are scaled and center-cropped to fill each monitor without letterboxing

## Technology

- **Rust** (2024 edition)
- **eframe / egui** for the GUI
- **image** crate for loading, resizing, and composing wallpapers
- **rayon** for parallel image processing
- **xrandr** for monitor detection
- **gsettings** for applying the wallpaper on Cinnamon

## Built with ❤️

This project was built by a human and [Claude Code](https://claude.com/claude-code) working together.
