# Changelog

All notable changes to Foxix terminal will be documented in this file.

## [0.1.0] - 2026-04-13

### Added
- Official name: **Foxix**
- GPU rendering with cell-sprites (OpenGL 4.6)
- Mouse selection + Wayland clipboard integration
- Braille/Block rasterization
- True color 24-bit + 256 colors support
- Kitty-style configuration (`foxix.conf`)
- Tabs and Splits system with multiple layouts
- Marks system for quick directory navigation
- Shell integration (bash/zsh/fish bootstrap)
- Notifications system
- Kittens (plugins): ssh, diff, panel, query, remote, bookmark
- Nerd Fonts support with proper cell height
- KGP (Kitty Graphics Protocol) implementation
- Transparency with wallpaper background extraction
- Benchmarks comparison script

### Fixed
- Nerd Font icons being cut off (cell height adjustment)
- Scrollback buffer size

### Performance
- ~12 MB RAM usage
- ~20ms startup time
- Zero runtime dependencies (100% Rust)