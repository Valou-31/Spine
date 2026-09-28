use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Rasterizes assets/logo.svg into a PNG at OUT_DIR/tray_icon.png so app.rs can
/// embed it with `include_bytes!`. ffmpeg on this platform has no SVG decoder,
/// so we shell out to the built-in QuickLook thumbnailer instead (qlmanage -t
/// always produces a square thumbnail, which is what we want for an icon).
///
/// qlmanage's SVG thumbnailer always flattens transparency onto an opaque
/// white background, regardless of what the SVG itself specifies, so we
/// chroma-key the white back out to alpha afterwards.
fn main() {
    let svg_path = "assets/logo.svg";
    println!("cargo:rerun-if-changed={svg_path}");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let ql_dir = Path::new(&out_dir).join("logo_ql");
    let _ = fs::remove_dir_all(&ql_dir);
    fs::create_dir_all(&ql_dir).expect("failed to create temp dir for qlmanage output");

    let status = Command::new("qlmanage")
        .args(["-t", "-s", "256", "-o"])
        .arg(&ql_dir)
        .arg(svg_path)
        .status()
        .expect("failed to run qlmanage to rasterize assets/logo.svg");
    assert!(status.success(), "qlmanage exited with a failure status");

    let generated = ql_dir.join(format!(
        "{}.png",
        Path::new(svg_path).file_name().unwrap().to_string_lossy()
    ));

    let mut img = image::open(&generated)
        .unwrap_or_else(|e| panic!("failed to open qlmanage output {generated:?}: {e}"))
        .into_rgba8();
    strip_white_background(&mut img);

    let dest = Path::new(&out_dir).join("tray_icon.png");
    img.save(&dest)
        .unwrap_or_else(|e| panic!("failed to save processed tray icon to {dest:?}: {e}"));
}

/// qlmanage bakes the SVG onto opaque white. Fade pixels back to transparent
/// as they approach white, so the icon has a real transparent background
/// again instead of a visible white square.
fn strip_white_background(img: &mut image::RgbaImage) {
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
}
