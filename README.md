<div align="center">

<img src="assets/logo/logo.png" alt="Foxix Logo" width="120">

<p align="center">
<pre>
███████╗ ██████╗ ██╗  ██╗██╗██╗  ██╗
██╔════╝██╔═══██╗╚██╗██╔╝██║╚██╗██╔╝
█████╗  ██║   ██║ ╚███╔╝ ██║ ╚███╔╝ 
██╔══╝  ██║   ██║ ██╔██╗ ██║ ██╔██╗ 
██║     ╚██████╔╝██╔╝ ██╗██║██╔╝ ██╗
╚═╝      ╚═════╝ ╚═╝  ╚═╝╚═╝╚═╝  ╚═╝
</pre>
</p>

**Foxix** — Emulador de terminal ultra-rápido escrito en Rust 🦀

[![Rust](https://img.shields.io/badge/Rust-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![OpenGL](https://img.shields.io/badge/OpenGL-4.6-blue?style=flat-square&logo=opengl)](https://www.opengl.org/)
[![Wayland](https://img.shields.io/badge/Wayland-green?style=flat-square)](https://wayland.freedesktop.org/)
[![License](https://img.shields.io/badge/License-MIT-purple?style=flat-square)](LICENSE)

</div>

---

## ¿Qué es Foxix?

**Foxix** es un emulador de terminal de alto rendimiento escrito en Rust con renderizado GPU (OpenGL 4.6). Diseñado para ser **ligero, rápido y moderno**.

> *Fox* (zorro) + *ix* = velocidad + poder

---

## 🏃‍♂️ Rendimiento vs Kitty

| Métrica | Foxix | Kitty | Ventaja |
|---------|-------|------|---------|
| **RAM** | ~12 MB | ~70 MB | 🦊 5x menos |
| **Startup** | ~20 ms | ~200 ms | 🦊 10x más rápido |
| **Binario** | 4.9 MB | 88 KB + Python | 🦊 Todo-en-uno |
| **Dependencias** | 0 | Python 3.10+ | 🦊 Sin runtime |

```bash
# Ejecuta el benchmark
./bench/benchmark_v4.sh
```

---

## 🚀 Características

- 🦊 **Renderizado GPU** — OpenGL 4.6 con cell-sprites
- 🎨 **Transparencia real** — fondo semitransparente (wallpaper)
- 🔤 **Nerd Fonts** — iconos completos (nerd-font patched)
- 🖱️ **Mouse** — selección de texto con arrastrar
- 📋 **Wayland clipboard** — `wl-copy` / `wl-paste`
- 🌈 **True color** — 24-bit color support
- 📜 **10k líneas** — scrollback configurable

---

## 📦 Instalación

```bash
# Compilar
git clone https://github.com/tu-usuario/foxix.git
cd foxix
cargo build --release

# Ejecutar
./target/release/foxix
```

### Requisitos

```bash
# Arch Linux
sudo pacman -S rust freetype2 wayland

# Ubuntu/Debian  
sudo apt install rustup libfreetype-dev libwayland-dev
```

---

## ⚙️ Configuración

El archivo de config se crea automáticamente en `~/.config/foxix/foxix.conf`:

```conf
# Ejemplo de configuración
font_family             JetBrains Mono Nerd Font
font_size               12
background_opacity      0.78
window_padding_width    25
shell                   fish
scrollback_lines        10000
```

---

## 🆚 Foxix vs Kitty

| Característica | Foxix | Kitty |
|----------------|-------|-------|
| **Lenguaje** | Rust 🦀 | Python + C |
| **RAM** | ~12 MB | ~70 MB |
| **Startup** | ~20 ms | ~200 ms |
| **GPU Rendering** | ✅ OpenGL 4.6 | ✅ OpenGL |
| **True Color** | ✅ | ✅ |
| **Nerd Fonts** | ✅ | ✅ |
| **Transparencia** | ✅ | ✅ |
| **Mouse Selection** | ✅ | ✅ |
| **Tabs** | 🟡 En desarrollo | ✅ |
| **Splits** | ❌ | ✅ |
| **Image Protocol** | ⚠️ WIP | ✅ |

---

## 🐧 Plataformas

| Plataforma | Estado |
|------------|--------|
| Linux (Wayland) | ✅ Soportado |
| Linux (X11) | 🟡 Parcial |
| macOS | ❌ No soportado |
| Windows | ❌ No soportado |

Optimizado para **Hyprland**, **Sway**, **GNOME Wayland**.

---

## 📜 Changelog

### v0.1.0
- 🦊 Nombre: Foxix
- ⚙️ Config estilo kitty.conf
- 🎨 Cell-sprites GPU
- 🖱️ Mouse + clipboard
- 🔷 Braille/Block rasterization
- 🌈 True color + 256 colores

---

## 📄 Licencia

[MIT © 2026 — Foxix Terminal](LICENSE)

---

<div align="center">

*Tan rápido como un zorro, tan sólido como un sistema.*  
🦊 **Fox** + **ix**

</div>
