<!-- SEO: Foxix terminal emulator Rust fast GPU OpenGL Wayland Linux -->
<div align="center">
<img src="assets/logo/logo.png" alt="Foxix Terminal — Fast Rust Terminal Emulator" width="120">

<pre>
███████╗ ██████╗ ██╗  ██╗██╗██╗  ██╗
██╔════╝██╔═══██╗╚██╗██╔╝██║╚██╗██╔╝
█████╗  ██║   ██║ ╚███╔╝ ██║ ╚███╔╝ 
██╔══╝  ██║   ██║ ██╔██╗ ██║ ██╔██╗ 
██║     ╚██████╔╝██╔╝ ██╗██║██╔╝ ██╗
╚═╝      ╚═════╝ ╚═╝  ╚═╝╚═╝╚═╝  ╚═╝
</pre>

# Foxix Terminal

**Foxix** — Emulador de terminal ultra-rápido escrito en Rust 🦀  
*A blazing-fast GPU-accelerated terminal emulator built with Rust and OpenGL*

[![Rust](https://img.shields.io/badge/Rust-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![OpenGL](https://img.shields.io/badge/OpenGL-4.6-blue?style=flat-square&logo=opengl)](https://www.opengl.org/)
[![Wayland](https://img.shields.io/badge/Wayland-green?style=flat-square)](https://wayland.freedesktop.org/)
[![License](https://img.shields.io/badge/License-MIT-purple?style=flat-square)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.1.0-red?style=flat-square)](CHANGELOG.md)

</div>

---

> 💡 **Foxix** es un emulador de terminal de alto rendimiento para Linux — escrito completamente en Rust, con renderizado por GPU (OpenGL 4.6), sin dependencias de runtime y optimizado para entornos Wayland como Hyprland y Sway.

---

## ¿Qué es Foxix?

**Foxix** es un terminal emulador moderno, ligero y rápido escrito en **Rust**, con aceleración GPU mediante **OpenGL 4.6**. Nació como alternativa minimalista a terminales como Kitty, Alacritty o WezTerm — consumiendo una fracción de la memoria RAM y arrancando hasta **10 veces más rápido**.

> *Fox* (zorro) + *ix* = velocidad + poder Unix

Si buscas un **terminal rápido para Linux**, un **emulador de terminal en Rust**, o un reemplazo ligero para Kitty/Alacritty en **Hyprland o Wayland**, Foxix es para ti.

---

## ⚡ Rendimiento — Foxix vs Kitty vs Alacritty

Foxix está diseñado para ser el terminal más eficiente disponible en Linux. Benchmarks reales comparando **Foxix terminal** con los emuladores más populares:

| Métrica | **Foxix** | Kitty | Alacritty |
|---------|-----------|-------|-----------|
| **RAM** | ~12 MB | ~70 MB | ~30 MB |
| **Startup** | ~20 ms | ~200 ms | ~50 ms |
| **Binario** | 4.9 MB | 88 KB + Python | ~7 MB |
| **Dependencias runtime** | **0** | Python 3.10+ | 0 |

🦊 Foxix usa **5× menos RAM** que Kitty y arranca **10× más rápido**.

```bash
# Ejecuta el benchmark tú mismo
./bench/benchmark_v4.sh
```

---

## 🚀 Características de Foxix Terminal

- 🦊 **Renderizado GPU** — OpenGL 4.6 con cell-sprites para máxima velocidad
- 🎨 **Transparencia real** — fondo semitransparente que muestra el wallpaper
- 🔤 **Nerd Fonts** — iconos completos con fuentes Nerd Font patched
- 🖱️ **Selección con mouse** — drag-to-select nativo
- 📋 **Wayland clipboard** — integración con `wl-copy` / `wl-paste`
- 🌈 **True color 24-bit** — soporte completo de 16M colores
- 📜 **Scrollback 10k líneas** — configurable
- ⚙️ **Config estilo Kitty** — migración sencilla desde `kitty.conf`
- 🦀 **100% Rust** — sin Python, sin runtime externo, sin sorpresas

---

## 📦 Instalación de Foxix

### Desde código fuente (recomendado)

```bash
# Clonar el repositorio de Foxix
git clone https://github.com/jephersonRD/foxix.git
cd foxix

# Compilar con Rust (release)
cargo build --release

# Ejecutar Foxix terminal
./target/release/foxix
```

### Requisitos del sistema

```bash
# Arch Linux / Manjaro / EndeavourOS
sudo pacman -S rust freetype2 wayland

# Ubuntu / Debian / Pop!_OS
sudo apt install rustup libfreetype-dev libwayland-dev

# Fedora
sudo dnf install rust freetype-devel wayland-devel
```

> Rust 1.70+ recomendado. Sin Python, sin dependencias de runtime.

---

## ⚙️ Configuración de Foxix

El archivo de configuración se genera automáticamente en `~/.config/foxix/foxix.conf`:

```conf
# Foxix Terminal — Configuración
font_family             JetBrains Mono Nerd Font
font_size               12
background_opacity      0.78
window_padding_width    25
shell                   fish
scrollback_lines        10000
```

La sintaxis es compatible con el estilo de `kitty.conf`, facilitando la migración.

---

## 🆚 Foxix vs otros terminales en Rust/Linux

| Característica | **Foxix** | Kitty | Alacritty | WezTerm |
|----------------|-----------|-------|-----------|---------|
| **Lenguaje** | Rust 🦀 | Python + C | Rust 🦀 | Rust 🦀 |
| **RAM** | ~12 MB | ~70 MB | ~30 MB | ~50 MB |
| **Startup** | ~20 ms | ~200 ms | ~50 ms | ~80 ms |
| **GPU Rendering** | ✅ OpenGL 4.6 | ✅ | ✅ | ✅ |
| **True Color** | ✅ | ✅ | ✅ | ✅ |
| **Nerd Fonts** | ✅ | ✅ | ✅ | ✅ |
| **Transparencia** | ✅ | ✅ | ⚠️ | ✅ |
| **Mouse Selection** | ✅ | ✅ | ✅ | ✅ |
| **Wayland nativo** | ✅ | ✅ | ✅ | ✅ |
| **Sin dependencias** | ✅ | ❌ Python | ✅ | ✅ |
| **Tabs** | 🟡 WIP | ✅ | ❌ | ✅ |
| **Splits** | ❌ | ✅ | ❌ | ✅ |
| **Image Protocol** | 🟡 WIP | ✅ | ❌ | ✅ |

---

## 🐧 Plataformas soportadas

| Plataforma | Estado |
|------------|--------|
| **Linux — Wayland** (Hyprland, Sway, GNOME) | ✅ Soportado |
| Linux — X11 | 🟡 Soporte parcial |
| macOS | ❌ No soportado aún |
| Windows | ❌ No soportado aún |

Foxix está **optimizado para Wayland**. Funciona perfectamente en **Hyprland**, **Sway** y **GNOME Wayland**. Si usas Arch Linux con Hyprland, Foxix es tu terminal.

---

## ❓ FAQ — Preguntas frecuentes sobre Foxix

**¿Foxix es más rápido que Alacritty?**  
Sí. Foxix usa ~60% menos RAM que Alacritty y tiene un tiempo de inicio menor gracias a su arquitectura sin runtime externo.

**¿Foxix funciona en Hyprland?**  
Sí, Foxix está diseñado y optimizado específicamente para Wayland, incluyendo Hyprland y Sway.

**¿Puedo migrar mi config de Kitty a Foxix?**  
La sintaxis de `foxix.conf` es compatible con el estilo de `kitty.conf`. La mayoría de opciones básicas son portables directamente.

**¿Por qué Rust para un emulador de terminal?**  
Rust garantiza memoria segura sin garbage collector, lo que se traduce en bajo uso de RAM y tiempos de inicio mínimos — perfectos para un terminal de alto rendimiento.

---

## 📜 Changelog

### v0.1.0 — Primera release de Foxix Terminal
- 🦊 Nombre oficial: **Foxix**
- ⚙️ Config estilo `kitty.conf`
- 🎨 Renderizado GPU con cell-sprites
- 🖱️ Mouse selection + Wayland clipboard
- 🔷 Braille/Block rasterization
- 🌈 True color 24-bit + 256 colores

---

## 🏷️ Topics / Tags

`terminal` `terminal-emulator` `rust` `opengl` `wayland` `hyprland` `linux` `gpu-rendering` `fast-terminal` `rust-terminal` `foxix` `kitty-alternative` `alacritty-alternative` `nerd-fonts` `true-color`

> 💡 Si estás en GitHub, agrega estos topics desde *Settings → Topics* para mejorar la visibilidad de **Foxix terminal** en búsquedas.

---

## 📄 Licencia

[MIT © 2026 — Foxix Terminal](LICENSE)

---

<div align="center">

*Tan rápido como un zorro, tan sólido como el sistema.*

🦊 **Foxix** — The fast Rust terminal emulator for Linux

[⭐ Star Foxix en GitHub](https://github.com/jephersonRD/foxix) · [🐛 Reportar bug](https://github.com/jephersonRD/foxix/issues) · [💬 Discusiones](https://github.com/jephersonRD/foxix/discussions)

</div>
