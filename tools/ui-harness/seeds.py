#!/usr/bin/env python3
"""Sample snips for screenshots, one set per language.

Everything in them is made up — no real document, account or person — as
revisetouch.com requires of product screenshots. Each seed comes with the
window title the gallery shows under it.

    seeds.py OUT_DIR [SCALE]   # writes OUT_DIR/{en,tr}/s1..s10.png and titles.json

SCALE draws them at that many physical pixels per logical one, as a snip
taken on a HiDPI screen would be (default 1).
"""
import json
import os
import sys

from PIL import Image, ImageDraw, ImageFont

SANS = "/usr/share/fonts/noto/NotoSans-Regular.ttf"
MONO = "/usr/share/fonts/noto/NotoSansMono-Regular.ttf"
K = 1

TEXT = {
    "en": {
        "hello": ("Hello World — Snipdeck", "Screen snips you can search by their text"),
        "check": ("Release checklist", ["[x] Linux build on Ubuntu 22.04", "[x] Windows software renderer",
                                        "[ ] Screenshots for the website", "[ ] Tag v0.3.0"], "Ship it"),
        "chart": "Snips per week, Q3",
        "dialog": ("Save changes before closing?", "Your edits to “Untitled” will be lost.", "Don’t save", "Save"),
        "table": ("Region", "Q2", "Q3", [("North", "1,240", "1,410"), ("South", "980", "1,105"), ("East", "1,530", "1,488")]),
        "notes": ("Meeting notes", ["Ship the Linux build first", "Website: new screenshots", "Ask for Windows testers"]),
        "titles": ["Sales.ods — Spreadsheet", "Palette — Colors", "Untitled — Editor", "Meeting notes — Notes",
                   "Snipdeck — README.md", "main.rs — Code", "Brand colours — Figma",
                   "Weekly report — Spreadsheet", "Release checklist — Notes", "cargo test — Terminal"],
    },
    "tr": {
        "hello": ("Merhaba Dünya — Snipdeck", "Metninden aranabilen ekran alıntıları"),
        "check": ("Yayın listesi", ["[x] Ubuntu 22.04'te Linux derlemesi", "[x] Windows yazılım çizicisi",
                                    "[ ] Site için ekran görüntüleri", "[ ] v0.3.0 etiketi"], "Yayınla"),
        "chart": "3. çeyrek, haftalık snip",
        "dialog": ("Kapatmadan önce kaydedilsin mi?", "“Adsız” belgesindeki değişiklikler kaybolacak.", "Kaydetme", "Kaydet"),
        "table": ("Bölge", "2. ç.", "3. ç.", [("Kuzey", "1.240", "1.410"), ("Güney", "980", "1.105"), ("Doğu", "1.530", "1.488")]),
        "notes": ("Toplantı notları", ["Önce Linux sürümü çıksın", "Site: yeni ekran görüntüleri", "Windows test edenler bulunsun"]),
        "titles": ["Satış.ods — Tablo", "Palet — Renkler", "Adsız — Düzenleyici", "Toplantı notları — Notlar",
                   "Snipdeck — README.md", "main.rs — Code", "Marka renkleri — Figma",
                   "Haftalık rapor — Tablo", "Yayın listesi — Notlar", "cargo test — Uçbirim"],
    },
}


def k(*v):
    """Logical to physical pixels: one number, or a tuple of them."""
    r = tuple(round(x * K) for x in v)
    return r[0] if len(r) == 1 else r


def font(path, size):
    return ImageFont.truetype(path, k(size))


def canvas(w, h, bg):
    im = Image.new("RGB", k(w, h), bg)
    return im, ImageDraw.Draw(im)


def hello(t):
    im, d = canvas(560, 140, "#ffffff")
    d.text(k(24, 34), t[0], font=font(SANS, 30), fill="#1f2328")
    d.text(k(24, 84), t[1], font=font(SANS, 17), fill="#57606a")
    return im


def code():
    im, d = canvas(480, 260, "#1e1f2e")
    f = font(MONO, 17)
    lines = [("fn main() {", "#89b4fa", 0), ('println!("hello");', "#a6e3a1", 1),
             ("let snips = 126;", "#f38ba8", 1), ("}", "#89b4fa", 0)]
    for i, (s, c, ind) in enumerate(lines):
        d.text(k(20 + ind * 26, 22 + i * 30), s, font=f, fill=c)
    return im


def shapes():
    im, d = canvas(360, 240, "#ff8a3d")
    d.polygon([k(360, 0), k(360, 240), k(0, 240)], fill="#7c3aed")
    d.ellipse(k(70, 70, 170, 170), fill="#ffffff")
    d.rectangle(k(200, 60, 300, 160), fill="#5b21b6")
    return im


def chart(title):
    im, d = canvas(420, 300, "#f6f5f0")
    d.text(k(26, 18), title, font=font(SANS, 19), fill="#1f2328")
    for i, h in enumerate([60, 120, 170, 230]):
        x = 40 + i * 92
        d.rectangle(k(x, 280 - h, x + 52, 280), fill="#ff8a3d" if i == 3 else "#3b82f6")
    return im


def checklist(t):
    title, items, button = t
    im, d = canvas(640, 400, "#ffffff")
    d.rectangle(k(0, 0, 640, 52), fill="#f1f3f5")
    d.text(k(22, 12), title, font=font(SANS, 22), fill="#1f2328")
    f = font(SANS, 18)
    for i, s in enumerate(items):
        d.text(k(32, 80 + i * 40), s, font=f, fill="#343a40")
    d.rectangle(k(32, 262, 232, 300), fill="#ff8a3d")
    d.text(k(50, 268), button, font=font(SANS, 17), fill="#ffffff")
    return im


def terminal():
    im, d = canvas(520, 320, "#111827")
    d.rectangle(k(0, 0, 520, 34), fill="#1f2937")
    f = font(MONO, 15)
    d.text(k(14, 8), "~/snipdeck — cargo test", font=f, fill="#9ca3af")
    d.text(k(14, 54), "test result: ok. 80 passed; 0 failed", font=f, fill="#4ade80")
    d.text(k(14, 86), "Finished release [optimized]", font=f, fill="#e5e7eb")
    d.text(k(14, 118), "warning: 0", font=f, fill="#fbbf24")
    return im


def dialog(t):
    title, body, no, yes = t
    im, d = canvas(520, 200, "#f6f6f6")
    d.text(k(24, 26), title, font=font(SANS, 20), fill="#1f2328")
    d.text(k(24, 64), body, font=font(SANS, 15), fill="#57606a")
    d.rounded_rectangle(k(250, 132, 370, 170), k(6), fill="#e4e6e9")
    d.text(k(268, 140), no, font=font(SANS, 15), fill="#1f2328")
    d.rounded_rectangle(k(382, 132, 496, 170), k(6), fill="#3b82f6")
    d.text(k(410, 140), yes, font=font(SANS, 15), fill="#ffffff")
    return im


def table(t):
    head, a, b, rows = t
    im, d = canvas(460, 210, "#ffffff")
    f = font(SANS, 16)
    d.rectangle(k(0, 0, 460, 40), fill="#e8f0e8")
    for x, s in ((20, head), (220, a), (340, b)):
        d.text(k(x, 10), s, font=f, fill="#1f2328")
    for i, (r, p, q) in enumerate(rows):
        y = 52 + i * 50
        d.line(k(0, y - 6, 460, y - 6), fill="#dde2dd", width=k(1))
        for x, s in ((20, r), (220, p), (340, q)):
            d.text(k(x, y + 6), s, font=f, fill="#343a40")
    return im


def palette():
    im, d = canvas(480, 180, "#ffffff")
    for i, c in enumerate(["#ff8a3d", "#7c3aed", "#3b82f6", "#10b981", "#1f2328"]):
        x = 16 + i * 92
        d.rounded_rectangle(k(x, 16, x + 80, 116), k(8), fill=c)
        d.text(k(x + 6, 128), c, font=font(MONO, 14), fill="#343a40")
    return im


def notes(t):
    title, items = t
    im, d = canvas(440, 230, "#fff8dc")
    d.text(k(22, 18), title, font=font(SANS, 21), fill="#3b3200")
    for i, s in enumerate(items):
        d.text(k(26, 70 + i * 44), "•  " + s, font=font(SANS, 16), fill="#4a4100")
    return im


def main():
    global K
    out = sys.argv[1]
    K = float(sys.argv[2]) if len(sys.argv) > 2 else 1
    titles = {}
    for lang, t in TEXT.items():
        os.makedirs(os.path.join(out, lang), exist_ok=True)
        # s1 is the oldest snip, so it sits last in the gallery; the newest
        # four (terminal, checklist, chart, shapes) make up the first row.
        images = [table(t["table"]), palette(), dialog(t["dialog"]), notes(t["notes"]),
                  hello(t["hello"]), code(), shapes(), chart(t["chart"]), checklist(t["check"]), terminal()]
        for i, im in enumerate(images, 1):
            im.save(os.path.join(out, lang, f"s{i}.png"))
        titles[lang] = t["titles"]
    json.dump(titles, open(os.path.join(out, "titles.json"), "w"), ensure_ascii=False, indent=1)


if __name__ == "__main__":
    main()
