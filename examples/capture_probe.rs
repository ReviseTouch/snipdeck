//! Quick capture sanity probe: enumerate monitors via xcap and grab one
//! frame from each. Used to validate the capture backend on new platforms
//! (first target: Linux/X11 under WSLg) without driving the full app.
//!
//!     cargo run --example capture_probe

fn main() {
    let monitors = match xcap::Monitor::all() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Monitor::all() FAILED: {e}");
            std::process::exit(1);
        }
    };
    if monitors.is_empty() {
        eprintln!("no monitors reported");
        std::process::exit(1);
    }
    for m in monitors {
        // xcap 0.0.14 (the pinned version) returns plain values here, same as
        // src/capture.rs uses them. Newer xcap (>= 0.4) makes these Result.
        let name = m.name().to_string();
        let (w, h) = (m.width(), m.height());
        print!("monitor {name}: {w}x{h} ... ");
        match m.capture_image() {
            Ok(img) => {
                // A capture that "succeeds" but comes back all-black is the
                // classic failure mode — check for any non-zero pixel.
                let non_zero = img.pixels().any(|p| p.0[0] != 0 || p.0[1] != 0 || p.0[2] != 0);
                println!(
                    "captured {}x{} ({})",
                    img.width(),
                    img.height(),
                    if non_zero { "has content" } else { "ALL BLACK" }
                );
            }
            Err(e) => println!("capture FAILED: {e}"),
        }
    }
}
