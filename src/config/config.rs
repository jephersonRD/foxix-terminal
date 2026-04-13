use anyhow::{Context, Result};
use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────────────────────
// Config de Foxix — formato compatible con kitty.conf
// Archivo: ~/.config/foxix/foxix.conf
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Config {
    // ── Fuente ────────────────────────────────────────────────────────────
    pub font_family:        String,
    pub bold_font:          String,   // "auto" | ruta
    pub italic_font:        String,   // "auto" | ruta
    pub bold_italic_font:   String,   // "auto" | ruta
    pub font_size:          u32,
    pub bold_is_bright:     bool,
    pub letter_spacing:     f32,

    // ── Cursor ───────────────────────────────────────────────────────────
    pub cursor_shape:             CursorShape,
    pub cursor_color:             Option<[u8; 3]>,   // None = usa fg color
    pub cursor_text_color:        Option<[u8; 3]>,   // color del texto bajo cursor
    pub cursor_beam_thickness:    f32,
    pub cursor_underline_thickness: f32,
    pub cursor_blink:             bool,
    pub cursor_blink_interval:    f32,
    pub cursor_stop_blinking_after: f32,

    // ── Ventana ──────────────────────────────────────────────────────────
    pub background_opacity:       f32,
    pub window_padding_width:     f32,   // padding todos los lados
    pub initial_window_width:     u32,
    pub initial_window_height:    u32,

    // ── Colores base ─────────────────────────────────────────────────────
    pub foreground:              [u8; 3],
    pub background:              [u8; 3],
    pub selection_foreground:    Option<[u8; 3]>,
    pub selection_background:    [u8; 3],

    // ── Paleta ANSI (color0..color15) ────────────────────────────────────
    pub color_palette:           [[u8; 3]; 16],

    // ── Shell ────────────────────────────────────────────────────────────
    pub shell: String,

    // ── Historial ────────────────────────────────────────────────────────
    pub scrollback_lines:        usize,

    // ── URLs ─────────────────────────────────────────────────────────────
    pub url_color:               [u8; 3],
    pub url_style:               UrlStyle,     // none | curly | single | double | strikethrough

    // ── Mouse ────────────────────────────────────────────────────────────
    pub mouse_hide_wait:         f32,   // segundos; -1 = nunca, 0 = inmediato

    // ── Campana ──────────────────────────────────────────────────────────
    pub enable_audio_bell:       bool,

    // ── Interno (calculado, no parseado) ─────────────────────────────────
    pub cell_padding:            f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CursorShape { Block, Underline, Beam }

#[derive(Debug, Clone, PartialEq)]
pub enum UrlStyle { None, Curly, Single, Double, Strikethrough }

// ── Tabla de colores por defecto (paleta Dracula-inspired) ────────────────
fn default_palette() -> [[u8; 3]; 16] {
    [
        [0x28, 0x29, 0x3f], // color0  (black)         — fondo oscuro
        [0xff, 0x55, 0x55], // color1  (red)
        [0x50, 0xfa, 0x7b], // color2  (green)
        [0xf1, 0xfa, 0x8c], // color3  (yellow)
        [0xbd, 0x93, 0xf9], // color4  (blue)           — lavanda Dracula
        [0xff, 0x79, 0xc6], // color5  (magenta)        — rosa Dracula
        [0x8b, 0xe9, 0xfd], // color6  (cyan)
        [0xf8, 0xf8, 0xf2], // color7  (white)
        [0x44, 0x47, 0x5a], // color8  (bright black)   — comentarios Dracula
        [0xff, 0x6e, 0x6e], // color9  (bright red)
        [0x69, 0xff, 0x94], // color10 (bright green)
        [0xff, 0xff, 0xa5], // color11 (bright yellow)
        [0xd6, 0xac, 0xff], // color12 (bright blue)
        [0xff, 0x92, 0xdf], // color13 (bright magenta)
        [0xa4, 0xff, 0xff], // color14 (bright cyan)
        [0xff, 0xff, 0xff], // color15 (bright white)
    ]
}

impl Default for Config {
    fn default() -> Self {
        Config {
            font_family:               "JetBrains Mono Nerd Font".to_string(),
            bold_font:                 "auto".to_string(),
            italic_font:               "auto".to_string(),
            bold_italic_font:          "auto".to_string(),
            font_size:                 12,
            bold_is_bright:            false,
            letter_spacing:            0.0,
            cursor_shape:              CursorShape::Beam,
            cursor_color:              None,
            cursor_text_color:         Some([0x11, 0x11, 0x11]),
            cursor_beam_thickness:     1.5,
            cursor_underline_thickness: 2.0,
            cursor_blink:              true,
            cursor_blink_interval:     0.5,
            cursor_stop_blinking_after: 15.0,
            background_opacity:        0.78,
            window_padding_width:      25.0,
            initial_window_width:      800,
            initial_window_height:     600,
            foreground:                [0xf8, 0xf8, 0xf2],
            background:                [0x1e, 0x1f, 0x29],
            selection_foreground:      None,  // None = invertir fg/bg como Kitty
            selection_background:      [0x44, 0x47, 0x5a],
            color_palette:             default_palette(),
            shell:                     shell_default(),
            scrollback_lines:          10000,
            url_color:                 [0x00, 0x87, 0xbd],
            url_style:                 UrlStyle::Curly,
            mouse_hide_wait:           3.0,
            enable_audio_bell:         false,
            cell_padding:              25.0,
        }
    }
}

fn shell_default() -> String {
    if std::path::Path::new("/usr/bin/fish").exists() {
        return "/usr/bin/fish".to_string();
    }
    if std::path::Path::new("/usr/bin/zsh").exists() {
        return "/usr/bin/zsh".to_string();
    }
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
}

impl Config {
    // ── Carga pública ──────────────────────────────────────────────────────

    pub fn load() -> Result<Self> {
        let path = Self::default_path();

        if !path.exists() {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, DEFAULT_CONF)
                .context("No se pudo crear ~/.config/foxix/foxix.conf")?;
            log::info!("Creado foxix.conf por defecto en: {:?}", path);
        }

        let content = std::fs::read_to_string(&path)
            .context(format!("No se pudo leer {:?}", path))?;

        let mut cfg = Self::default();
        cfg.parse_conf(&content);
        cfg.cell_padding = cfg.window_padding_width;

        // ── Paleta del wallpaper ──────────────────────────────────────────
        // Si el usuario NO tiene colores forzados en foxix.conf, detectar
        // automáticamente desde el wallpaper actual (como pywal/wal)
        if let Some(wp) = crate::config::wallpaper::extract_wallpaper_palette() {
            log::info!("Foxix: paleta auto-detectada desde {:?}", wp.source_path.file_name().unwrap_or_default());
            cfg.color_palette  = wp.colors;
            cfg.background     = wp.background;
            cfg.foreground     = wp.foreground;
        }

        log::info!(
            "Foxix config: fuente={} {}pt | shell={} | opacidad={:.2} | padding={}px",
            cfg.font_family, cfg.font_size,
            cfg.shell, cfg.background_opacity, cfg.window_padding_width
        );

        Ok(cfg)
    }

    pub fn default_path() -> PathBuf {
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| dirs_home().join(".config"));
        base.join("foxix").join("foxix.conf")
    }

    // ── Parser de formato Kitty ────────────────────────────────────────────
    // Soporta: "key value", comentarios "#", comentarios inline, líneas vacías

    fn parse_conf(&mut self, content: &str) {
        for raw in content.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') { continue; }

            let mut parts = line.splitn(2, |c: char| c == ' ' || c == '\t');
            let key = match parts.next() { Some(k) => k.trim(), None => continue };
            let rest = match parts.next() { Some(v) => v.trim(), None => continue };
            // Eliminar comentario inline
            let val = rest.splitn(2, '#').next().unwrap_or(rest).trim();
            if val.is_empty() { continue; }

            match key {
                // Fuente
                "font_family"             => self.font_family = val.to_string(),
                "bold_font"               => self.bold_font = val.to_string(),
                "italic_font"             => self.italic_font = val.to_string(),
                "bold_italic_font"        => self.bold_italic_font = val.to_string(),
                "font_size"               => self.font_size = val.parse().unwrap_or(self.font_size),
                "bold_is_bright"          => self.bold_is_bright = parse_bool(val),
                "letter_spacing"          => self.letter_spacing = val.parse().unwrap_or(0.0),

                // Cursor
                "cursor"                  => self.cursor_color = parse_hex_color(val),
                "cursor_text_color"       => self.cursor_text_color = parse_hex_color(val),
                "cursor_shape"            => {
                    self.cursor_shape = match val.to_lowercase().as_str() {
                        "beam"      => CursorShape::Beam,
                        "underline" => CursorShape::Underline,
                        _           => CursorShape::Block,
                    };
                }
                "cursor_beam_thickness"        => self.cursor_beam_thickness = val.parse().unwrap_or(1.5),
                "cursor_underline_thickness"   => self.cursor_underline_thickness = val.parse().unwrap_or(2.0),
                "cursor_blink_interval"        => {
                    let v: f32 = val.parse().unwrap_or(0.5);
                    self.cursor_blink = v > 0.0;
                    self.cursor_blink_interval = v.abs();
                }
                "cursor_stop_blinking_after"   => self.cursor_stop_blinking_after = val.parse().unwrap_or(15.0),

                // Ventana
                "background_opacity"      => self.background_opacity = val.parse().unwrap_or(0.78),
                "window_padding_width"    => {
                    let first = val.split_whitespace().next().unwrap_or("25");
                    self.window_padding_width = first.parse().unwrap_or(25.0);
                }
                "initial_window_width"    => self.initial_window_width = val.parse().unwrap_or(800),
                "initial_window_height"   => self.initial_window_height = val.parse().unwrap_or(600),

                // Colores base
                "foreground"              => { if let Some(c) = parse_hex_color(val) { self.foreground = c; } }
                "background"              => { if let Some(c) = parse_hex_color(val) { self.background = c; } }
                "selection_foreground"    => self.selection_foreground = parse_hex_color(val),
                "selection_background"    => { if let Some(c) = parse_hex_color(val) { self.selection_background = c; } }

                // Paleta ANSI color0..color15
                k if k.starts_with("color") => {
                    if let Ok(n) = k["color".len()..].parse::<usize>() {
                        if n < 16 {
                            if let Some(c) = parse_hex_color(val) {
                                self.color_palette[n] = c;
                            }
                        }
                    }
                }

                // Shell
                "shell"                   => {
                    let bin = val.split_whitespace().next().unwrap_or(val);
                    self.shell = resolve_shell(bin);
                }

                // Historial
                "scrollback_lines"        => self.scrollback_lines = val.parse().unwrap_or(10000),

                // URLs
                "url_color"               => { if let Some(c) = parse_hex_color(val) { self.url_color = c; } }
                "url_style"               => {
                    self.url_style = match val.to_lowercase().as_str() {
                        "none"          => UrlStyle::None,
                        "curly"         => UrlStyle::Curly,
                        "single"        => UrlStyle::Single,
                        "double"        => UrlStyle::Double,
                        "strikethrough" => UrlStyle::Strikethrough,
                        _               => UrlStyle::Curly,
                    };
                }

                // Mouse
                "mouse_hide_wait"         => self.mouse_hide_wait = val.parse().unwrap_or(3.0),

                // Campana
                "enable_audio_bell"       => self.enable_audio_bell = parse_bool(val),

                // Ignorar keys desconocidas (compatibilidad con kitty.conf real)
                _ => {}
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_bool(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "yes" | "true" | "1" | "on")
}

/// Parsea colores en formato #RRGGBB o #RGB
fn parse_hex_color(s: &str) -> Option<[u8; 3]> {
    let s = s.trim_start_matches('#');
    match s.len() {
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some([r, g, b])
        }
        3 => {
            let r = u8::from_str_radix(&s[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&s[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&s[2..3], 16).ok()? * 17;
            Some([r, g, b])
        }
        _ => None,
    }
}

fn resolve_shell(name: &str) -> String {
    if name.starts_with('/') { return name.to_string(); }
    for prefix in &["/usr/bin", "/usr/local/bin", "/bin"] {
        let path = format!("{}/{}", prefix, name);
        if std::path::Path::new(&path).exists() {
            return path;
        }
    }
    name.to_string()
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

// ─────────────────────────────────────────────────────────────────────────────
// Config por defecto — se escribe en ~/.config/foxix/foxix.conf
// ─────────────────────────────────────────────────────────────────────────────
const DEFAULT_CONF: &str = r#"# ╔══════════════════════════════════════════════════════════════════════╗
# ║                Foxix Terminal — foxix.conf                           ║
# ║  Formato compatible con kitty.conf: clave valor  # comentario        ║
# ╚══════════════════════════════════════════════════════════════════════╝

# ── Fuente ─────────────────────────────────────────────────────────────────
font_family             JetBrains Mono Nerd Font
bold_font               auto
italic_font             auto
bold_italic_font        auto
font_size               12
bold_is_bright          no
letter_spacing          0

# ── Cursor ─────────────────────────────────────────────────────────────────
# cursor               #cccccc          # color del cursor (defecto = color fg)
cursor_shape            beam            # beam | block | underline
cursor_beam_thickness   1.5
cursor_underline_thickness 2.0
cursor_blink_interval   0.5            # segundos; 0 = sin parpadeo
cursor_stop_blinking_after 15.0        # dejar de parpadear tras N segundos idle

# ── Ventana ────────────────────────────────────────────────────────────────
background_opacity      0.78           # 0.0 transparente | 1.0 sólido
window_padding_width    25             # padding en píxeles (todos los lados)
initial_window_width    800
initial_window_height   600

# ── Colores base ───────────────────────────────────────────────────────────
foreground              #f8f8f2
background              #1e1f29
selection_foreground    #ffffff
selection_background    #44475a

# ── Paleta ANSI (Dracula) ──────────────────────────────────────────────────
color0   #21222c
color1   #ff5555
color2   #50fa7b
color3   #f1fa8c
color4   #bd93f9
color5   #ff79c6
color6   #8be9fd
color7   #f8f8f2
color8   #6272a4
color9   #ff6e6e
color10  #69ff94
color11  #ffffa5
color12  #d6acff
color13  #ff92df
color14  #a4ffff
color15  #ffffff

# ── Shell ──────────────────────────────────────────────────────────────────
shell                   fish           # fish | zsh | bash | /ruta/al/shell

# ── Historial ──────────────────────────────────────────────────────────────
scrollback_lines        10000

# ── URLs ───────────────────────────────────────────────────────────────────
url_color               #0087bd
url_style               curly          # none | curly | single | double | strikethrough

# ── Mouse ──────────────────────────────────────────────────────────────────
mouse_hide_wait         3.0            # segundos hasta esconder el cursor; -1 = nunca

# ── Campana ────────────────────────────────────────────────────────────────
enable_audio_bell       no
"#;
