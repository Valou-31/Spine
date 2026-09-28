use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Rasterizes an SVG into a square PNG via the built-in QuickLook
/// thumbnailer. ffmpeg on this platform has no SVG decoder, so this is the
/// only readily available renderer. `qlmanage -t` always produces a *square*
/// thumbnail, padding with opaque white around content that doesn't fill it;
/// callers that care about the content's own aspect ratio should trim that
/// padding back off afterwards (see `trim_to_content`).
fn rasterize_svg(svg_path: &str, out_dir: &Path, size: u32) -> PathBuf {
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

    let dest = out_dir.join(format!(
        "{}.png",
        Path::new(svg_path).file_stem().unwrap().to_string_lossy()
    ));
    fs::copy(&generated, &dest)
        .unwrap_or_else(|e| panic!("failed to copy rasterized {svg_path} to {dest:?}: {e}"));
    dest
}

/// Crops an image down to the tight bounding box of its non-white content,
/// discarding any surrounding padding. Used instead of guessing where a
/// renderer anchors its padding (which can vary) - measuring the actual
/// pixels is the only way to get a crop that's reliably centered.
fn trim_to_content(path: &Path) {
    let img = image::open(path)
        .unwrap_or_else(|e| panic!("failed to open {path:?} for trimming: {e}"))
        .into_rgba8();
    let (w, h) = img.dimensions();
    const THRESHOLD: u8 = 250;
    let is_content =
        |x: u32, y: u32| img.get_pixel(x, y).0[..3].iter().min().copied().unwrap() < THRESHOLD;

    let (mut min_x, mut max_x, mut min_y, mut max_y) = (w, 0, h, 0);
    for y in 0..h {
        for x in 0..w {
            if is_content(x, y) {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    assert!(min_x <= max_x && min_y <= max_y, "{path:?} has no non-white content");

    image::DynamicImage::ImageRgba8(img)
        .crop_imm(min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
        .save(path)
        .unwrap_or_else(|e| panic!("failed to save trimmed {path:?}: {e}"));
}

/// Center-crops an image to a target aspect ratio (width / height). Used to
/// make the two flag icons share the exact same aspect ratio so they both
/// fill an identically-sized UI button with no letterboxing gap - otherwise
/// whichever flag's native ratio doesn't match the button ends up looking
/// off-center/incomplete next to the other.
fn center_crop_to_aspect(path: &Path, target_aspect: f32) {
    let img =
        image::open(path).unwrap_or_else(|e| panic!("failed to open {path:?} for cropping: {e}"));
    let (w, h) = (img.width(), img.height());
    let current_aspect = w as f32 / h as f32;
    let cropped = if current_aspect > target_aspect {
        let new_w = (h as f32 * target_aspect).round() as u32;
        img.crop_imm((w - new_w) / 2, 0, new_w, h)
    } else {
        let new_h = (w as f32 / target_aspect).round() as u32;
        img.crop_imm(0, (h - new_h) / 2, w, new_h)
    };
    cropped
        .save(path)
        .unwrap_or_else(|e| panic!("failed to save cropped {path:?}: {e}"));
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
    println!("cargo:rerun-if-changed=assets/france.png");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    // App/tray icon: square with transparency around the logo shape.
    let tray_icon = rasterize_svg("assets/logo.svg", &out_dir, 256);
    strip_white_background(&tray_icon);
    fs::rename(&tray_icon, out_dir.join("tray_icon.png")).unwrap();

    // UK flag for the language toggle: fully opaque, so trim straight to the
    // rendered content's bounding box rather than assuming where qlmanage
    // put its padding - that keeps the flag centered either way.
    let gb_flag = rasterize_svg("assets/gb.svg", &out_dir, 240);
    trim_to_content(&gb_flag);
    // Match the France flag's own aspect ratio so both fill the same UI
    // button size with no gap on either one.
    let (fr_w, fr_h) = image::image_dimensions("assets/france.png")
        .expect("failed to read assets/france.png dimensions");
    center_crop_to_aspect(&gb_flag, fr_w as f32 / fr_h as f32);
    fs::rename(&gb_flag, out_dir.join("gb_flag.png")).unwrap();
}
