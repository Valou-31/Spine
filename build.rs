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
    println!("cargo:rerun-if-changed={svg_path}");

    let ql_dir = out_dir.join(format!(
        "{}_ql",
        Path::new(svg_path).file_stem().unwrap().to_string_lossy()
    ));
    let _ = fs::remove_dir_all(&ql_dir);
    fs::create_dir_all(&ql_dir).expect("failed to create temp dir for qlmanage output");

    let status = Command::new("qlmanage")
        .args(["-t", "-s", &size.to_string(), "-o"])
        .arg(&ql_dir)
        .arg(svg_path)
        .status()
        .unwrap_or_else(|e| panic!("failed to run qlmanage on {svg_path}: {e}"));
    assert!(status.success(), "qlmanage exited with a failure status");

    let generated = ql_dir.join(format!(
        "{}.png",
        Path::new(svg_path).file_name().unwrap().to_string_lossy()
    ));

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

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    // App/tray icon: square with transparency around the logo shape.
    let tray_icon = rasterize_svg("assets/logo.svg", &out_dir, 256, 1.0);
    strip_white_background(&tray_icon);
    fs::rename(&tray_icon, out_dir.join("tray_icon.png")).unwrap();

    // UK flag for the language toggle: fully opaque, viewBox is 50x30 (no
    // white stripping needed, just cropping off qlmanage's padding).
    let gb_flag = rasterize_svg("assets/gb.svg", &out_dir, 240, 50.0 / 30.0);
    fs::rename(&gb_flag, out_dir.join("gb_flag.png")).unwrap();
}
