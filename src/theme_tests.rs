//! The main window's tokens (ui/theme.slint) keep the rules they were derived
//! by. A colour nudged by hand — "this grey looks a bit light" — passes review
//! one nudge at a time; this fails the first one that breaks a rule.

use std::collections::HashMap;

/// `name -> (dark, light)` from `out property <color> name: dark ? #x : #y;`,
/// and `name -> (c, c)` from a single colour.
fn tokens() -> HashMap<String, (String, String)> {
    let src = include_str!("../ui/theme.slint");
    let mut out = HashMap::new();
    for line in src.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("out property <color> ") else {
            continue;
        };
        let Some((name, value)) = rest.split_once(':') else { continue };
        let hexes: Vec<String> = value
            .split(|c: char| !(c.is_ascii_hexdigit() || c == '#'))
            .filter(|t| t.starts_with('#'))
            .map(str::to_string)
            .collect();
        let pair = match hexes.as_slice() {
            [d, l] => (d.clone(), l.clone()),
            [c] => (c.clone(), c.clone()),
            _ => continue,
        };
        out.insert(name.trim().to_string(), pair);
    }
    out
}

fn luminance(hex: &str) -> f64 {
    let h = hex.trim_start_matches('#');
    let channel = |i: usize| {
        let c = u8::from_str_radix(&h[i..i + 2], 16).unwrap() as f64 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(0) + 0.7152 * channel(2) + 0.0722 * channel(4)
}

fn contrast(a: &str, b: &str) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

fn each_theme(check: impl Fn(&str, &dyn Fn(&str) -> String)) {
    let t = tokens();
    for (theme, dark) in [("dark", true), ("light", false)] {
        let get = |name: &str| -> String {
            let (d, l) = t.get(name).unwrap_or_else(|| panic!("no token {name}"));
            if dark { d.clone() } else { l.clone() }
        };
        check(theme, &get);
    }
}

#[test]
fn inks_hold_their_contrast_on_every_surface() {
    // Targets 3.1 / 5.2 / 10.5 / 16.5 against the ground; the floors below
    // leave room for the panel and raised surfaces, which sit a step away.
    each_theme(|theme, get| {
        for surface in ["ground", "panel", "raised"] {
            for (ink, floor) in [("ink-4", 2.9), ("ink-3", 4.5), ("ink-2", 9.5), ("ink-1", 15.0)] {
                let c = contrast(&get(ink), &get(surface));
                assert!(c >= floor, "{theme}: {ink} on {surface} is {c:.2}, under {floor}");
            }
        }
    });
}

#[test]
fn surfaces_step_in_one_direction() {
    // Light: from the raised card down to the strongest line. Dark: the
    // ground is the darkest and every step after it lighter.
    each_theme(|theme, get| {
        let ladder = if theme == "light" {
            ["raised", "ground", "panel", "hover", "fill", "selected", "line", "line-2", "island", "line-strong"]
        } else {
            ["ground", "panel", "raised", "hover", "fill", "selected", "line", "line-2", "island", "line-strong"]
        };
        let lum: Vec<f64> = ladder.iter().map(|n| luminance(&get(n))).collect();
        for (i, pair) in lum.windows(2).enumerate() {
            let ok = if theme == "light" { pair[1] < pair[0] } else { pair[1] > pair[0] };
            assert!(ok, "{theme}: {} → {} breaks the ladder", ladder[i], ladder[i + 1]);
        }
    });
}

#[test]
fn the_accent_and_danger_inks_are_legible() {
    each_theme(|theme, get| {
        let on_accent = contrast(&get("on-accent"), &get("accent"));
        assert!(on_accent >= 4.5, "{theme}: ink on the accent is {on_accent:.2}");
        let danger = contrast(&get("danger"), &get("raised"));
        assert!(danger >= 4.5, "{theme}: danger text on raised is {danger:.2}");
        let on_danger = contrast(&get("on-danger"), &get("danger-fill"));
        assert!(on_danger >= 4.5, "{theme}: ink on the danger fill is {on_danger:.2}");
    });
}
