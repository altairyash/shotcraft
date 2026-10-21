use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
use serde::Serialize;
use std::{fs, path::{Path, PathBuf}};

#[derive(Parser, Debug)]
#[command(name = "shotcraft", version, about = "Deterministic screenshot capture, composition, and validation")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// List built-in output presets.
    Presets,
    /// Inspect an image's dimensions, format, and aspect ratio.
    Inspect { input: PathBuf, #[arg(long)] json: bool },
    /// Validate an image against a preset's exact dimensions.
    Validate { input: PathBuf, #[arg(long, value_enum)] preset: PresetName, #[arg(long)] json: bool },
    /// Fit an image to a preset canvas and write a deterministic output.
    Compose {
        input: PathBuf,
        #[arg(long, value_enum)] preset: PresetName,
        #[arg(long, value_enum, default_value_t = FitMode::Contain)] fit: FitMode,
        #[arg(long, default_value = "#111827")] background: String,
        #[arg(short, long)] output: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
enum PresetName { Article, GithubReadme, Video16x9, Social }

#[derive(Clone, Copy, Debug, ValueEnum)]
enum FitMode { Contain, Cover }

#[derive(Clone, Copy, Debug, Serialize)]
struct Preset { name: &'static str, width: u32, height: u32, description: &'static str }

impl PresetName {
    fn preset(self) -> Preset {
        match self {
            Self::Article => Preset { name: "article", width: 1200, height: 630, description: "Article and link preview" },
            Self::GithubReadme => Preset { name: "github-readme", width: 1600, height: 900, description: "GitHub README project visual" },
            Self::Video16x9 => Preset { name: "video-16x9", width: 1920, height: 1080, description: "Full HD video frame" },
            Self::Social => Preset { name: "social", width: 1200, height: 630, description: "Social sharing card" },
        }
    }
}

#[derive(Serialize)]
struct ImageReport {
    path: String,
    format: String,
    width: u32,
    height: u32,
    aspect_ratio: f64,
    bytes: u64,
}

#[derive(Serialize)]
struct ValidationReport {
    valid: bool,
    path: String,
    preset: String,
    expected_width: u32,
    expected_height: u32,
    actual_width: Option<u32>,
    actual_height: Option<u32>,
    errors: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Presets => {
            for name in [PresetName::Article, PresetName::GithubReadme, PresetName::Video16x9, PresetName::Social] {
                let p = name.preset();
                println!("{:<14} {:>4}x{:<4} {}", p.name, p.width, p.height, p.description);
            }
        }
        Command::Inspect { input, json } => {
            let report = inspect(&input)?;
            if json { println!("{}", serde_json::to_string_pretty(&report)?); }
            else { println!("{}\n{}x{} ({:.3}:1)\n{} bytes", report.path, report.width, report.height, report.aspect_ratio, report.bytes); }
        }
        Command::Validate { input, preset, json } => {
            let p = preset.preset();
            let decoded = image::open(&input);
            let (width, height, errors) = match decoded {
                Ok(image) => {
                    let (w, h) = image.dimensions();
                    let mut errors = Vec::new();
                    if w != p.width || h != p.height { errors.push(format!("Expected {}x{}, got {}x{}", p.width, p.height, w, h)); }
                    (Some(w), Some(h), errors)
                }
                Err(e) => (None, None, vec![format!("Cannot decode image: {e}")]),
            };
            let report = ValidationReport { valid: errors.is_empty(), path: input.display().to_string(), preset: p.name.to_string(), expected_width: p.width, expected_height: p.height, actual_width: width, actual_height: height, errors };
            if json { println!("{}", serde_json::to_string_pretty(&report)?); }
            else if report.valid { println!("PASS {} matches {} ({}x{})", report.path, p.name, p.width, p.height); }
            else { eprintln!("FAIL {}", report.path); for error in &report.errors { eprintln!("  - {error}"); } }
            if !report.valid { std::process::exit(1); }
        }
        Command::Compose { input, preset, fit, background, output } => {
            let p = preset.preset();
            let source = image::open(&input).with_context(|| format!("Could not open input image {}", input.display()))?;
            let composed = compose(source, p.width, p.height, fit, parse_color(&background)?)?;
            if let Some(parent) = output.parent() { if !parent.as_os_str().is_empty() { fs::create_dir_all(parent)?; } }
            composed.save_with_format(&output, ImageFormat::Png).with_context(|| format!("Could not write {}", output.display()))?;
            println!("Wrote {} ({}x{})", output.display(), p.width, p.height);
        }
    }
    Ok(())
}

fn inspect(path: &Path) -> Result<ImageReport> {
    let image = image::open(path).with_context(|| format!("Could not decode image {}", path.display()))?;
    let (width, height) = image.dimensions();
    let format = ImageFormat::from_path(path).map(|f| format!("{f:?}")).unwrap_or_else(|_| "unknown".into());
    let bytes = fs::metadata(path)?.len();
    Ok(ImageReport { path: path.display().to_string(), format, width, height, aspect_ratio: width as f64 / height as f64, bytes })
}

fn compose(source: DynamicImage, width: u32, height: u32, fit: FitMode, background: Rgba<u8>) -> Result<DynamicImage> {
    if width == 0 || height == 0 { bail!("Output dimensions must be greater than zero"); }
    let (sw, sh) = source.dimensions();
    let scale = match fit { FitMode::Contain => (width as f64 / sw as f64).min(height as f64 / sh as f64), FitMode::Cover => (width as f64 / sw as f64).max(height as f64 / sh as f64) };
    let rw = ((sw as f64 * scale).round() as u32).max(1);
    let rh = ((sh as f64 * scale).round() as u32).max(1);
    let resized = source.resize_exact(rw, rh, FilterType::Lanczos3).to_rgba8();
    let mut canvas = RgbaImage::from_pixel(width, height, background);
    let x = (width as i64 - rw as i64) / 2;
    let y = (height as i64 - rh as i64) / 2;
    for sy in 0..rh {
        for sx in 0..rw {
            let dx = sx as i64 + x;
            let dy = sy as i64 + y;
            if dx >= 0 && dy >= 0 && dx < width as i64 && dy < height as i64 {
                let pixel = resized.get_pixel(sx, sy);
                let alpha = pixel[3] as f32 / 255.0;
                let dest = canvas.get_pixel_mut(dx as u32, dy as u32);
                for c in 0..3 { dest[c] = (pixel[c] as f32 * alpha + dest[c] as f32 * (1.0 - alpha)).round() as u8; }
                dest[3] = 255;
            }
        }
    }
    Ok(DynamicImage::ImageRgba8(canvas))
}

fn parse_color(value: &str) -> Result<Rgba<u8>> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() != 6 { bail!("Background must be a hex color like #111827"); }
    let r = u8::from_str_radix(&hex[0..2], 16).context("Invalid background hex color")?;
    let g = u8::from_str_radix(&hex[2..4], 16).context("Invalid background hex color")?;
    let b = u8::from_str_radix(&hex[4..6], 16).context("Invalid background hex color")?;
    Ok(Rgba([r, g, b, 255]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presets_have_expected_dimensions() {
        assert_eq!((PresetName::Article.preset().width, PresetName::Article.preset().height), (1200, 630));
        assert_eq!((PresetName::Video16x9.preset().width, PresetName::Video16x9.preset().height), (1920, 1080));
    }
    #[test]
    fn parses_hex_background() { assert_eq!(parse_color("#112233").unwrap(), Rgba([17, 34, 51, 255])); }
}
