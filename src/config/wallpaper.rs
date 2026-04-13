/// Foxix Wallpaper Color Extractor
/// ──────────────────────────────────────────────────────────────────────────
/// Detecta el wallpaper actual y extrae una paleta de 16 colores usando
/// K-means de 3 iteraciones sobre píxeles submuestreados.
/// Compatible con: caelestia/swww, hyprpaper, feh, swaybg, XFCE, GNOME.
use std::path::PathBuf;

/// Paleta generada del wallpaper — 16 colores asignados igual que color0-15
#[derive(Debug, Clone)]
pub struct WallpaperPalette {
    /// color0-15 en formato sRGB
    pub colors: [[u8; 3]; 16],
    /// Fondo oscuro derivado del color dominante más oscuro
    pub background: [u8; 3],
    /// Texto claro derivado del color dominante más claro
    pub foreground: [u8; 3],
    /// Ruta del wallpaper que generó esta paleta
    pub source_path: PathBuf,
}

/// Intenta detectar y extraer una paleta del wallpaper actual.
/// Devuelve None si no puede encontrar o decodificar el wallpaper.
pub fn extract_wallpaper_palette() -> Option<WallpaperPalette> {
    let path = detect_wallpaper_path()?;
    log::info!("Foxix wallpaper: {:?}", path);
    extract_palette_from_path(&path)
}

/// Extrae la paleta de un archivo de imagen en disco
pub fn extract_palette_from_path(path: &std::path::Path) -> Option<WallpaperPalette> {
    // Cargar y submuestrar la imagen a 80×80 para velocidad
    let img = image::open(path).ok()?;
    let small = img.resize_exact(80, 80, image::imageops::FilterType::Nearest);
    let rgb = small.to_rgb8();

    // Recoger todos los píxeles
    let pixels: Vec<[f32; 3]> = rgb
        .pixels()
        .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
        .collect();

    // K-means con k=16, 4 iteraciones
    let centers = kmeans_16(&pixels, 4);

    // Ordenar los centros por luminancia perceptual (BT.601)
    let mut sorted = centers;
    sorted.sort_by(|a, b| {
        luma(a).partial_cmp(&luma(b)).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Asignar a colour0..colour15
    let mut colors = [[0u8; 3]; 16];

    // Primeros 8: versión normal (oscuros → brillantes)
    for i in 0..8 {
        let idx = i * 2; // saltar de 2 en 2 para mejor distribución
        let src = sorted[idx.min(sorted.len() - 1)];
        colors[i] = [src[0] as u8, src[1] as u8, src[2] as u8];
    }
    // Últimos 8: versión "bright" (50% más saturados y brillantes)
    for i in 0..8 {
        colors[i + 8] = brighten(&colors[i], 1.4);
    }

    // Asegurar que color0 sea muy oscuro (fondo) y color7/15 muy claros (texto)
    colors[0] = darken(&sorted[0].map(|x| x as u8), 0.4);
    colors[7] = brighten(&sorted[sorted.len() - 1].map(|x| x as u8), 1.1);
    colors[8] = darken(&colors[7], 0.7);
    colors[15] = [240, 240, 250]; // casi blanco fijo

    // background = color dominante muy oscuro
    let dom = sorted[0];
    let background = darken(&[dom[0] as u8, dom[1] as u8, dom[2] as u8], 0.3);

    // foreground = complementario del dominante, muy claro
    let foreground = [235u8, 220, 210]; // crema suave legible sobre cualquier fondo

    Some(WallpaperPalette {
        colors,
        background,
        foreground,
        source_path: path.to_path_buf(),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Detección del wallpaper en varios entornos
// ─────────────────────────────────────────────────────────────────────────────

fn detect_wallpaper_path() -> Option<PathBuf> {
    // 1. Variable de entorno FOXIX_WALLPAPER (override manual del usuario)
    if let Ok(p) = std::env::var("FOXIX_WALLPAPER") {
        let path = PathBuf::from(p);
        if path.exists() { return Some(path); }
    }

    // 2. swww query (más común en Hyprland)
    if let Ok(out) = std::process::Command::new("swww").arg("query").output() {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            // Formato: "output: HDMI-A-1, image: /path/to/image.jpg, ..."
            if let Some(pos) = line.find("image: ") {
                let rest = &line[pos + 7..];
                let end = rest.find(',').unwrap_or(rest.len());
                let path = PathBuf::from(rest[..end].trim());
                if path.exists() { return Some(path); }
            }
        }
    }

    // 3. Caelestia: puede guardar el wallpaper en ~/.local/share/caelestia/
    let caelestia_wall = dirs_home().join(".local/share/caelestia/wallpaper");
    if caelestia_wall.exists() {
        if let Ok(link) = std::fs::read_link(&caelestia_wall) {
            if link.exists() { return Some(link); }
        }
        return Some(caelestia_wall);
    }

    // 4. hyprpaper listloaded
    if let Ok(out) = std::process::Command::new("hyprctl")
        .args(["hyprpaper", "listloaded"])
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        let first = text.lines().next().map(|l| l.trim().to_string());
        if let Some(p) = first {
            let path = PathBuf::from(&p);
            if path.exists() { return Some(path); }
        }
    }

    // 5. feh --bg-fill (guarda en ~/.fehbg)
    let fehbg = dirs_home().join(".fehbg");
    if fehbg.exists() {
        if let Ok(content) = std::fs::read_to_string(&fehbg) {
            for token in content.split_whitespace() {
                if token.contains('/') {
                    let path = PathBuf::from(token.trim_matches('\'').trim_matches('"'));
                    if path.exists() { return Some(path); }
                }
            }
        }
    }

    // 6. Buscar el wallpaper más reciente en ~/Pictures/Wallpapers/
    let wallpaper_dirs = [
        dirs_home().join("Pictures/Wallpapers"),
        dirs_home().join("Pictures"),
        dirs_home().join(".config/wallpapers"),
    ];
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for dir in &wallpaper_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    if matches!(ext.to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "webp" | "bmp") {
                        if let Ok(meta) = p.metadata() {
                            if let Ok(mtime) = meta.modified() {
                                if newest.is_none() || mtime > newest.as_ref().unwrap().0 {
                                    newest = Some((mtime, p));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some((_, path)) = newest {
        return Some(path);
    }

    None
}

// ─────────────────────────────────────────────────────────────────────────────
// K-means simplificado (k=16, n iteraciones)
// ─────────────────────────────────────────────────────────────────────────────

fn kmeans_16(pixels: &[[f32; 3]], iters: usize) -> Vec<[f32; 3]> {
    let k = 16usize;
    if pixels.is_empty() { return vec![[0.0; 3]; k]; }

    // Inicializar centros con K-means++ simplificado: equidistribuidos
    let step = pixels.len() / k;
    let mut centers: Vec<[f32; 3]> = (0..k).map(|i| pixels[(i * step).min(pixels.len() - 1)]).collect();

    for _ in 0..iters {
        // Asignar cada píxel al centro más cercano
        let mut sums = vec![[0.0f64; 3]; k];
        let mut counts = vec![0usize; k];

        for px in pixels {
            let best = (0..k)
                .min_by(|&a, &b| {
                    dist2(px, &centers[a])
                        .partial_cmp(&dist2(px, &centers[b]))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(0);
            sums[best][0] += px[0] as f64;
            sums[best][1] += px[1] as f64;
            sums[best][2] += px[2] as f64;
            counts[best] += 1;
        }

        // Recentrar
        for i in 0..k {
            if counts[i] > 0 {
                centers[i] = [
                    (sums[i][0] / counts[i] as f64) as f32,
                    (sums[i][1] / counts[i] as f64) as f32,
                    (sums[i][2] / counts[i] as f64) as f32,
                ];
            }
        }
    }

    centers
}

#[inline]
fn dist2(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    let dr = a[0] - b[0];
    let dg = a[1] - b[1];
    let db = a[2] - b[2];
    dr * dr + dg * dg + db * db
}

/// Luminancia perceptual BT.601
#[inline]
fn luma(rgb: &[f32; 3]) -> f32 {
    0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2]
}

/// Oscurecer un color (factor < 1.0)
fn darken(rgb: &[u8; 3], factor: f32) -> [u8; 3] {
    [
        (rgb[0] as f32 * factor).clamp(0.0, 255.0) as u8,
        (rgb[1] as f32 * factor).clamp(0.0, 255.0) as u8,
        (rgb[2] as f32 * factor).clamp(0.0, 255.0) as u8,
    ]
}

/// Aclarar/saturar un color (factor > 1.0)
fn brighten(rgb: &[u8; 3], factor: f32) -> [u8; 3] {
    [
        (rgb[0] as f32 * factor).clamp(0.0, 255.0) as u8,
        (rgb[1] as f32 * factor).clamp(0.0, 255.0) as u8,
        (rgb[2] as f32 * factor).clamp(0.0, 255.0) as u8,
    ]
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
