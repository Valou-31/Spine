use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Rasterizes an SVG into a PNG via the built-in QuickLook thumbnailer. ffmpeg
/// on this platform has no SVG decoder, so this is the only readily available
/// renderer. `qlmanage -t` always produces a *square* thumbnail: if the SVG's
/// content doesn't fill it, it scales to fill the full width and pads the
/// remaining height with opaque white, top-anchored. `content_aspect` (width
/// / height of the SVG's own viewBox) lets us crop that padding back off.
fn rasterize_svg(svg_path: &str, out_dir: &Path, size: u32, content_aspect: f32) -> PathBuf {
    let generated = run_qlmanage(svg_path, out_dir, size, false);

    let content_height = (size as f32 / content_aspect).round() as u32;
    let cropped = image::open(&generated)
        .unwrap_or_else(|e| panic!("failed to open qlmanage output {generated:?}: {e}"))
        .crop_imm(0, 0, size, content_height.min(size));

    let dest = out_dir.join(format!(
        "{}.png",
        Path::new(svg_path).file_stem().unwrap().to_string_lossy()
    ));
    cropped
        .save(&dest)
        .unwrap_or_else(|e| panic!("failed to save rasterized {svg_path} to {dest:?}: {e}"));
    dest
}

/// Rasterizes an SVG via QuickLook's *icon* mode (`-i`), then crops tightly
/// to the actual drawn content instead of trusting a hardcoded aspect-ratio
/// formula. Plain `-t` thumbnailing turned out to render `gb.svg`'s content
/// off-center within its own bounding box (the Union Jack's cross landed
/// ~15% right of center, not just padded) - icon mode centers it correctly,
/// so this is used instead of `rasterize_svg` for that asset. Cropping to
/// the measured content bounding box (rather than an assumed one) also makes
/// this robust to whatever framing/shadow padding icon mode itself adds.
fn rasterize_svg_icon_centered(svg_path: &str, out_dir: &Path, size: u32) -> PathBuf {
    let generated = run_qlmanage(svg_path, out_dir, size, true);

    let img = image::open(&generated)
        .unwrap_or_else(|e| panic!("failed to open qlmanage output {generated:?}: {e}"))
        .into_rgba8();
    let (min_x, min_y, max_x, max_y) = content_bbox(&img);
    let cropped = image::DynamicImage::ImageRgba8(img).crop_imm(
        min_x,
        min_y,
        max_x - min_x + 1,
        max_y - min_y + 1,
    );

    let dest = out_dir.join(format!(
        "{}.png",
        Path::new(svg_path).file_stem().unwrap().to_string_lossy()
    ));
    cropped
        .save(&dest)
        .unwrap_or_else(|e| panic!("failed to save rasterized {svg_path} to {dest:?}: {e}"));
    dest
}

fn run_qlmanage(svg_path: &str, out_dir: &Path, size: u32, icon_mode: bool) -> PathBuf {
    println!("cargo:rerun-if-changed={svg_path}");

    let ql_dir = out_dir.join(format!(
        "{}_ql",
        Path::new(svg_path).file_stem().unwrap().to_string_lossy()
    ));
    let _ = fs::remove_dir_all(&ql_dir);
    fs::create_dir_all(&ql_dir).expect("failed to create temp dir for qlmanage output");

    let mut args = vec!["-t".to_string(), "-s".to_string(), size.to_string()];
    if icon_mode {
        args.push("-i".to_string());
    }
    args.push("-o".to_string());

    let status = Command::new("qlmanage")
        .args(&args)
        .arg(&ql_dir)
        .arg(svg_path)
        .status()
        .unwrap_or_else(|e| panic!("failed to run qlmanage on {svg_path}: {e}"));
    assert!(status.success(), "qlmanage exited with a failure status");

    ql_dir.join(format!(
        "{}.png",
        Path::new(svg_path).file_name().unwrap().to_string_lossy()
    ))
}

/// Finds the bounding box of visibly-drawn content (saturated color, such as
/// the flag's navy/red) as opposed to the white page background and the
/// faint gray shadow that QuickLook's icon mode frames it with.
fn content_bbox(img: &image::RgbaImage) -> (u32, u32, u32, u32) {
    let (mut min_x, mut min_y) = (u32::MAX, u32::MAX);
    let (mut max_x, mut max_y) = (0u32, 0u32);
    for (x, y, pixel) in img.enumerate_pixels() {
        let [r, g, b, _a] = pixel.0;
        let (r, g, b) = (r as i32, g as i32, b as i32);
        let saturation = r.max(g).max(b) - r.min(g).min(b);
        if saturation > 30 {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    assert!(min_x <= max_x, "no saturated content found to crop to");
    (min_x, min_y, max_x, max_y)
}

/// Center-crops an image to a target aspect ratio (width / height), then
/// resizes it to the exact given pixel dimensions. Used to make the two flag
/// icons pixel-for-pixel identical in size, so there is zero ambiguity left
/// for egui's fit/centering logic - relying on both images merely sharing an
/// aspect ratio still leaves scale_to_fit to compute the on-screen size
/// independently for each, which can round differently.
fn center_crop_and_resize(path: &Path, target_width: u32, target_height: u32) {
    let img =
        image::open(path).unwrap_or_else(|e| panic!("failed to open {path:?} for cropping: {e}"));
    let (w, h) = (img.width(), img.height());
    let target_aspect = target_width as f32 / target_height as f32;
    let current_aspect = w as f32 / h as f32;
    let cropped = if current_aspect > target_aspect {
        let new_w = (h as f32 * target_aspect).round() as u32;
        img.crop_imm((w - new_w) / 2, 0, new_w, h)
    } else {
        let new_h = (w as f32 / target_aspect).round() as u32;
        img.crop_imm(0, (h - new_h) / 2, w, new_h)
    };
    let resized = cropped.resize_exact(
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    );
    resized
        .save(path)
        .unwrap_or_else(|e| panic!("failed to save resized {path:?}: {e}"));
}

/// qlmanage bakes the SVG onto opaque white. Fade pixels back to transparent
/// as they approach white, so the icon has a real transparent background
/// again instead of a visible white square.
fn strip_white_background(path: &Path) {
    let mut img = image::open(path)
        .unwrap_or_else(|e| panic!("failed to open {path:?} for alpha fix-up: {e}"))
        .into_rgba8();
    const THRESHOLD: f32 = 0.75;
    for pixel in img.pixels_mut() {
        let [r, g, b, a] = pixel.0;
        let whiteness = r.min(g).min(b) as f32 / 255.0;
        let keep = if whiteness <= THRESHOLD {
            1.0
        } else {
            ((1.0 - whiteness) / (1.0 - THRESHOLD)).clamp(0.0, 1.0)
        };
        pixel.0[3] = (a as f32 * keep).round() as u8;
    }
    img.save(path)
        .unwrap_or_else(|e| panic!("failed to save alpha-fixed {path:?}: {e}"));
}

/// Both flag buttons in the UI are displayed at this exact pixel size, so
/// both source images are normalized to it here rather than left to egui's
/// per-image fit calculation at render time.
const FLAG_WIDTH: u32 = 96;
const FLAG_HEIGHT: u32 = 64;

fn main() {
    println!("cargo:rerun-if-changed=assets/france.png");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    // App/tray icon: square with transparency around the logo shape.
    let tray_icon = rasterize_svg("assets/logo.svg", &out_dir, 256, 1.0);
    strip_white_background(&tray_icon);
    fs::rename(&tray_icon, out_dir.join("tray_icon.png")).unwrap();

    // UK flag for the language toggle: fully opaque, cropped to its actual
    // drawn content (see rasterize_svg_icon_centered for why plain -t
    // thumbnailing doesn't work for this asset).
    let gb_flag = rasterize_svg_icon_centered("assets/gb.svg", &out_dir, 240);
    center_crop_and_resize(&gb_flag, FLAG_WIDTH, FLAG_HEIGHT);
    fs::rename(&gb_flag, out_dir.join("gb_flag.png")).unwrap();

    // Normalize the France flag to the exact same pixel size as the GB flag.
    let fr_flag = out_dir.join("fr_flag.png");
    fs::copy("assets/france.png", &fr_flag).expect("failed to copy assets/france.png");
    center_crop_and_resize(&fr_flag, FLAG_WIDTH, FLAG_HEIGHT);
}
