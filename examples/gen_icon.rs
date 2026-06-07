//! Snipdeck icon generator.
//!
//! Renders the app icon — a selection corner-frame around an orange "snip
//! card" — procedurally (supersampled hard-edge coverage, downscaled with
//! Lanczos3 for anti-aliasing) and writes a multi-size `.ico` plus a few
//! preview PNGs. Run with: `cargo run --example gen_icon`.

use image::{imageops::FilterType, Rgba, RgbaImage};

const REF: f32 = 256.0; // design coordinate space
const SS: u32 = 4; // supersample factor
const MASTER: u32 = 256 * SS; // 1024

// Palette
const CARD_TOP: [f32; 4] = [255.0, 168.0, 92.0, 255.0]; // #ffa85c
const CARD_BOT: [f32; 4] = [243.0, 120.0, 32.0, 255.0]; // #f37820
const CARD_BORDER: [f32; 4] = [198.0, 92.0, 18.0, 255.0]; // #c65c12
const BRACKET: [f32; 4] = [140.0, 136.0, 127.0, 255.0]; // #8c887f bold warm gray
const EDGE: [f32; 4] = [30.0, 22.0, 14.0, 130.0]; // soft dark outline (for light bg)

fn lerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

/// `src` over `dst`, both straight-alpha [r,g,b,a] in 0..255.
fn over(dst: [f32; 4], src: [f32; 4]) -> [f32; 4] {
    let sa = src[3] / 255.0;
    let da = dst[3] / 255.0;
    let oa = sa + da * (1.0 - sa);
    if oa <= 0.0 {
        return [0.0, 0.0, 0.0, 0.0];
    }
    let mix = |s: f32, d: f32| (s * sa + d * da * (1.0 - sa)) / oa;
    [
        mix(src[0], dst[0]),
        mix(src[1], dst[1]),
        mix(src[2], dst[2]),
        oa * 255.0,
    ]
}

/// Signed distance to a rounded rect (negative inside).
fn rr_sd(px: f32, py: f32, cx: f32, cy: f32, hw: f32, hh: f32, r: f32) -> f32 {
    let qx = (px - cx).abs() - (hw - r);
    let qy = (py - cy).abs() - (hh - r);
    let ox = qx.max(0.0);
    let oy = qy.max(0.0);
    ox.hypot(oy) + qx.max(qy).min(0.0) - r
}

/// Distance from point to a line segment.
fn seg_dist(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (dx, dy) = (bx - ax, by - ay);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((px - ax) * dx + (py - ay) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (cx, cy) = (ax + t * dx, ay + t * dy);
    (px - cx).hypot(py - cy)
}

fn render_master() -> RgbaImage {
    // Selection frame corner brackets.
    let inset = 14.0;
    let arm = 44.0;
    let half_t = 10.5; // bracket half-thickness
    let edge_w = 2.0;
    let corners = [
        (inset, inset, 1.0, 1.0),               // TL
        (REF - inset, inset, -1.0, 1.0),         // TR
        (inset, REF - inset, 1.0, -1.0),         // BL
        (REF - inset, REF - inset, -1.0, -1.0),  // BR
    ];

    // Snip card.
    let (cx, cy) = (128.0, 128.0);
    let (hw, hh, r) = (62.0, 49.0, 17.0);
    let bw = 3.0; // border width

    let mut img = RgbaImage::new(MASTER, MASTER);
    for y in 0..MASTER {
        for x in 0..MASTER {
            let rx = (x as f32 + 0.5) / SS as f32;
            let ry = (y as f32 + 0.5) / SS as f32;
            let mut px = [0.0, 0.0, 0.0, 0.0];

            // Min distance to any bracket arm.
            let mut d = f32::MAX;
            for &(ox, oy, sx, sy) in &corners {
                d = d.min(seg_dist(rx, ry, ox, oy, ox + sx * arm, oy));
                d = d.min(seg_dist(rx, ry, ox, oy, ox, oy + sy * arm));
            }
            if d <= half_t + edge_w {
                px = over(px, EDGE);
            }
            if d <= half_t {
                px = over(px, BRACKET);
            }

            // Snip card (border + gradient fill).
            let sd_out = rr_sd(rx, ry, cx, cy, hw, hh, r);
            if sd_out <= 0.0 {
                let sd_in = rr_sd(rx, ry, cx, cy, hw - bw, hh - bw, r - bw);
                let color = if sd_in <= 0.0 {
                    let t = ((ry - (cy - hh)) / (2.0 * hh)).clamp(0.0, 1.0);
                    let mut c = lerp(CARD_TOP, CARD_BOT, t);
                    // soft top highlight
                    if ry < cy - hh * 0.25 {
                        c = lerp(c, [255.0, 255.0, 255.0, 255.0], 0.12);
                    }
                    c
                } else {
                    CARD_BORDER
                };
                px = over(px, color);
            }

            img.put_pixel(
                x,
                y,
                Rgba([
                    px[0].round() as u8,
                    px[1].round() as u8,
                    px[2].round() as u8,
                    px[3].round() as u8,
                ]),
            );
        }
    }
    img
}

fn main() -> anyhow::Result<()> {
    std::fs::create_dir_all("assets")?;
    let master = render_master();

    // Multi-size .ico from the high-res master.
    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in [16u32, 24, 32, 48, 64, 128, 256] {
        let resized = image::imageops::resize(&master, size, size, FilterType::Lanczos3);
        let icon = ico::IconImage::from_rgba_data(size, size, resized.into_raw());
        dir.add_entry(ico::IconDirEntry::encode(&icon)?);
    }
    let mut f = std::fs::File::create("assets/snipdeck.ico")?;
    dir.write(&mut f)?;

    // Preview PNGs for inspection.
    for size in [256u32, 64, 32, 16] {
        let resized = image::imageops::resize(&master, size, size, FilterType::Lanczos3);
        resized.save(format!("assets/preview_{size}.png"))?;
    }
    // Also a raw 32px RGBA the tray can embed directly later.
    let tray32 = image::imageops::resize(&master, 32, 32, FilterType::Lanczos3);
    std::fs::write("assets/tray_32.rgba", tray32.into_raw())?;

    println!("wrote assets/snipdeck.ico + preview PNGs");
    Ok(())
}
