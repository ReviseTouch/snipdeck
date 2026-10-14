#!/usr/bin/env python3
"""Product screenshots for revisetouch.com and the README, from a headless
GNOME session (run.sh), in every language and theme the site shows.

    shots.py OUT_DIR [--langs tr,en] [--themes dark,light] [--scenes a,b]

Writes OUT_DIR/<scene>-<lang>-<theme>.webp (and .png beside it). Each scene
is its own session; each language starts from an empty gallery seeded with
the made-up snips of seeds.py. Coordinates below were measured on a
1920x1200 logical monitor; SCALE (default 2) only multiplies the pixels.
"""
import argparse
import json
import os
import re
import shutil
import sqlite3
import subprocess
import sys
import uuid

HERE = os.path.dirname(os.path.abspath(__file__))
WORK = os.environ.get("WORK", os.path.join(os.environ.get("XDG_CACHE_HOME", os.path.expanduser("~/.cache")),
                                          "snipdeck-ui-harness"))
SCALE = int(os.environ.get("SCALE", "2"))
SEEDS = os.path.join(WORK, f"seeds@{SCALE}x")
DISMISS = [{"wait": 6000}, {"hideOverview": True}, {"wait": 2500},
           # The portal's "Add Keyboard Shortcuts" dialog: Cancel.
           {"click": {"win": "Add Keyboard Shortcuts", "x": 48, "y": 22}}, {"wait": 2500}]


def run(plan, **env):
    json.dump(plan, open(os.path.join(WORK, "plan.json"), "w"))
    e = dict(os.environ, WORK=WORK, VMON="1920x1200", SCALE=str(SCALE), REAL_DISPLAY=os.environ.get("REAL_DISPLAY", ":0"))
    e.update({k: str(v) for k, v in env.items()})
    subprocess.run([os.path.join(HERE, "run.sh")], env=e, check=False,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return open(os.path.join(WORK, "shell.log"), errors="replace").read()


def geometry(log, title):
    """Last logged x, y, w, h of the window called `title`."""
    hits = re.findall(r'ui: window "' + re.escape(title) + r'" (-?\d+),(-?\d+) (\d+)x(\d+)', log)
    hits = [tuple(map(int, h)) for h in hits if int(h[2]) > 0]
    if not hits:
        raise SystemExit(f"no window {title!r} in the log")
    return hits[-1]


def settings(lang, theme):
    path = os.path.join(WORK, "appconfig", "Snipdeck", "settings.json")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    json.dump({"language": lang, "dark_theme": theme == "dark"}, open(path, "w"))


def seed(lang):
    """An empty gallery with the seeds, each under a made-up window title."""
    shutil.rmtree(os.path.join(WORK, "appdata"), ignore_errors=True)
    titles = json.load(open(os.path.join(SEEDS, "titles.json")))[lang]
    files = " ".join(os.path.join(SEEDS, lang, f"s{i}.png") for i in range(1, len(titles) + 1))
    run(DISMISS + [{"wait": 1000}], SEEDS=files)
    db = sqlite3.connect(os.path.join(WORK, "appdata", "Snipdeck", "snips.db"))
    rows = db.execute("SELECT id, w, h FROM snips ORDER BY created_at").fetchall()
    if len(rows) != len(titles):
        raise SystemExit(f"seeding made {len(rows)} snips, not {len(titles)} — see {WORK}/app.log")
    for (sid, w, h), title in zip(rows, titles):
        db.execute(
            "INSERT OR REPLACE INTO snip_window (snip_id, platform, window_id, title, app_name, x, y, w, h,"
            " confidence, updated_at) VALUES (?, 'linux', ?, ?, ?, 0, 0, ?, ?, 1.0, datetime('now'))",
            (sid, uuid.uuid4().hex[:8], title, title.split(" — ")[-1], w, h))
    db.commit()
    db.close()
    shutil.rmtree(os.path.join(WORK, "appdata.seeded"), ignore_errors=True)
    shutil.copytree(os.path.join(WORK, "appdata"), os.path.join(WORK, "appdata.seeded"))


def restore():
    """Back to the seeded gallery: a scene that snips adds to it, and the next
    scene's cards would move."""
    shutil.rmtree(os.path.join(WORK, "appdata"), ignore_errors=True)
    shutil.copytree(os.path.join(WORK, "appdata.seeded"), os.path.join(WORK, "appdata"))


def save(src_png, box, out_base, round_top=0):
    """Crop a logical-pixel box out of a stage shot taken at SCALE.

    round_top: the radius of a window's rounded top corners, made transparent
    so the page's frame shows there instead of the desktop behind the window.
    """
    from PIL import Image, ImageDraw
    x, y, w, h = (v * SCALE for v in box)
    im = Image.open(src_png).convert("RGBA").crop((x, y, x + w, y + h))
    if round_top:
        r = round_top * SCALE
        mask = Image.new("L", im.size, 0)
        d = ImageDraw.Draw(mask)
        d.rounded_rectangle((0, 0, w - 1, h - 1), r, fill=255)
        d.rectangle((0, r, w - 1, h - 1), fill=255)
        im.putalpha(mask)
    im.save(out_base + ".png")
    subprocess.run(["magick", out_base + ".png", "-quality", "92", "-define", "webp:method=6",
                    out_base + ".webp"], check=True)


def shot(name):
    return os.path.join(WORK, "shots", name)


# ---- scenes -----------------------------------------------------------------

def scene_gallery(lang, theme, out):
    """The gallery, the pointer resting on the second card to show its actions."""
    log = run(DISMISS + [{"move": {"x": 830, "y": 380}}, {"wait": 1200},
                         {"windows": True}, {"shot": "gallery.png"}])
    save(shot("gallery.png"), geometry(log, "Snipdeck"), os.path.join(out, f"gallery-{lang}-{theme}"), round_top=12)


# A flat desktop under the scenes that show it, a step off the site's page
# colour so it still reads as a desktop.
DESKTOP = {"dark": "#141a24", "light": "#e6ebf0"}
TITLES = {"annotate": {"en": "Annotate", "tr": "İşaretle"}, "collage": {"en": "Collage", "tr": "Kolaj"}}
# The collage's Round slider knob: its label's width moves it.
ROUND_KNOB_X = {"en": 608, "tr": 627}
# The context menu opens with its corner at the pointer; its rows, measured
# from there.
MENU_ANNOTATE = (73, 255)
MENU_UPLOAD = (177, 162)
MENU_WIN_Y = 32  # the menu window starts below the top bar


def menu_click(px, py, off):
    return {"click": {"win": "SnipdeckContextMenu", "x": px + off[0], "y": py + off[1] - MENU_WIN_Y}}


def scene_floating(lang, theme, out):
    """Two floating snips on a plain desktop, one with its right-click menu."""
    floats = f"{SEEDS}/{lang}/s8.png:{860 * SCALE}:{150 * SCALE} {SEEDS}/{lang}/s10.png:{240 * SCALE}:{470 * SCALE}"
    log = run(DISMISS + [{"click": {"win": "Snipdeck", "x": 1082, "y": 17}}, {"wait": 1500},
                         {"move": {"x": 1010, "y": 330}}, {"wait": 700},
                         {"click": {"win": "SnipdeckFloating 17*", "x": 153, "y": 183, "button": 3}}, {"wait": 1500},
                         {"move": {"x": 1080, "y": 405}}, {"wait": 900},
                         {"windows": True}, {"shot": "floating.png"}],
              WATCHER=1, FLOATS=floats, BG=DESKTOP[theme])
    if "SnipdeckContextMenu" not in log:
        raise SystemExit("floating: the menu did not open")
    save(shot("floating.png"), (200, 110, 1120, 760), os.path.join(out, f"floating-{lang}-{theme}"))


# The annotation editor at SCALE 2 with the 640x400 checklist: tool pills
# along y=68, and the canvas measured from a shot (x 24-952, y 113-692).
ANNO_TOOLS = {"highlighter": 379, "rect": 422, "arrow": 551, "text": 594}
ANNO_CANVAS = (24, 113, 928 / 640)  # left, top, window px per checklist px


def on_canvas(x, y):
    """A point on the checklist (seeds.py's logical pixels) in the editor."""
    left, top, k = ANNO_CANVAS
    return round(left + x * k), round(top + y * k)


def scene_annotate(lang, theme, out):
    """The annotation editor on the checklist: a box round the open item, an
    arrow to it, a note, and the next item highlighted."""
    if theme != "dark":
        return
    a = TITLES["annotate"][lang]
    px, py = 825, 409  # the checklist, second card

    def tool(name):
        return [{"click": {"win": a, "x": ANNO_TOOLS[name], "y": 68}}, {"wait": 300}]

    def drag(p0, p1):
        (x0, y0), (x1, y1) = on_canvas(*p0), on_canvas(*p1)
        return [{"drag": {"win": a, "x": x0, "y": y0, "dx": x1 - x0, "dy": y1 - y0}}, {"wait": 400}]

    note_x, note_y = on_canvas(470, 62)
    plan = (DISMISS + [{"move": {"x": px, "y": py}}, {"wait": 500},
                       {"click": {"win": "Snipdeck", "x": px - 410, "y": py - 249, "button": 3}}, {"wait": 1500},
                       {"move": {"x": px + 60, "y": py + 240}}, {"wait": 500},
                       menu_click(px, py, MENU_ANNOTATE), {"wait": 3500}]
            + tool("rect") + drag((24, 150), (345, 192))
            + tool("arrow") + drag((560, 112), (362, 168))
            + tool("highlighter") + drag((58, 212), (200, 212))
            + tool("text") + [{"click": {"win": a, "x": note_x, "y": note_y}}, {"wait": 600},
                              {"type": "sonraki" if lang == "tr" else "next"}, {"key": "Return"}, {"wait": 800},
                              {"move": {"x": 1700, "y": 1100}}, {"wait": 600},
                              {"windows": True}, {"shot": "annotate.png"}])
    log = run(plan)
    save(shot("annotate.png"), geometry(log, a), os.path.join(out, f"annotate-{lang}-{theme}"), round_top=12)


def scene_collage(lang, theme, out):
    """The collage editor: five snips in justified rows, corners rounded."""
    if theme != "dark":
        return
    c = TITLES["collage"][lang]
    log = run(DISMISS + [{"click": {"win": "Snipdeck", "x": 150, "y": 57}}, {"wait": 3000},
                         {"click": {"win": c, "x": 424, "y": 710}}, {"wait": 800},
                         {"drag": {"win": c, "x": ROUND_KNOB_X[lang], "y": 800, "dx": 36, "dy": 0}}, {"wait": 800},
                         {"move": {"x": 1700, "y": 1100}}, {"wait": 600},
                         {"windows": True}, {"shot": "collage.png"}])
    save(shot("collage.png"), geometry(log, c), os.path.join(out, f"collage-{lang}-{theme}"), round_top=12)


def scene_upload(lang, theme, out):
    """The upload prompt over the gallery: where the snip goes and what that means."""
    px, py = 555, 409  # the first card
    log = run(DISMISS + [{"move": {"x": px, "y": py}}, {"wait": 500},
                         {"click": {"win": "Snipdeck", "x": px - 410, "y": py - 249, "button": 3}}, {"wait": 1500},
                         {"move": {"x": px + 145, "y": py + 151}}, {"wait": 500},
                         menu_click(px, py, MENU_UPLOAD), {"wait": 2500},
                         {"move": {"x": 1700, "y": 1100}}, {"wait": 600},
                         {"windows": True}, {"shot": "upload.png"}])
    if "SnipdeckUpload" not in log:
        raise SystemExit("upload: the window did not open")
    save(shot("upload.png"), geometry(log, "Snipdeck"), os.path.join(out, f"upload-{lang}-{theme}"), round_top=12)


# "Keep it on top", as a video: a chart is snipped with Super+Alt+drag in an
# image viewer, the snip is dragged aside, the text editor comes forward over
# the viewer — and the snip stays above it while a sentence is typed.
ONTOP_TEXT = {
    # Short lines: the snip is dragged over the editor's right half.
    "tr": ("Haftalık rapor — 3. çeyrek\n\nSnip’ler her hafta arttı.\n\n", "Grafik yanımda duruyor."),
    "en": ("Weekly report — Q3\n\nSnips grew every week.\n\n", "The chart stays beside me."),
}
ONTOP_AREA = (270, 180, 1200, 640)  # recorded, in logical pixels


def scene_ontop(lang, theme, out):
    """The video: snip, drag aside, bring another window over the source."""
    demo = os.path.join(WORK, "demo")
    shutil.rmtree(demo, ignore_errors=True)
    for d in ("demodata", "democonfig"):
        shutil.rmtree(os.path.join(WORK, d), ignore_errors=True)
    os.makedirs(demo)
    # The editor's text has to read at the size the video is shown.
    kf = os.path.join(WORK, "democonfig", "glib-2.0", "settings", "keyfile")
    os.makedirs(os.path.dirname(kf))
    open(kf, "w").write("[org/gnome/TextEditor]\nuse-system-font=false\ncustom-font='Monospace 17'\n"
                        "restore-session=false\nshow-line-numbers=false\n")
    body, typed = ONTOP_TEXT[lang]
    report = os.path.join(demo, "rapor.txt" if lang == "tr" else "report.txt")
    open(report, "w").write(body)
    chart = os.path.join(demo, "grafik.png" if lang == "tr" else "chart.png")
    shutil.copy(os.path.join(SEEDS, lang, "s8.png"), chart)
    x, y, w, h = ONTOP_AREA
    plan = [{"wait": 5000}, {"hideOverview": True},
            {"waitFor": "Add Keyboard Shortcuts", "timeout": 9000}, {"wait": 800},
            {"dismiss": {"win": "Add Keyboard Shortcuts", "x": 48, "y": 22}},
            {"place": {"win": os.path.basename(report) + "*", "x": 420, "y": 200, "w": 1040, "h": 600}},
            {"place": {"win": os.path.basename(chart) + "*", "x": 300, "y": 260, "w": 560, "h": 470, "raise": True}},
            {"move": {"x": 1700, "y": 1050}}, {"wait": 1500},
            {"record": {"x": x, "y": y, "w": w, "h": h, "file": "ontop"}}, {"wait": 700},
            # Snip the chart (x 370-790, y 368-668 in the viewer).
            {"glide": {"x": 362, "y": 360, "ms": 800}}, {"wait": 200},
            {"keyDown": "Super_L"}, {"keyDown": "Alt_L"}, {"wait": 250},
            {"button": {"press": True}}, {"glide": {"x": 798, "y": 676, "ms": 1100}}, {"wait": 150},
            {"button": {"press": False}}, {"wait": 150},
            {"keyUp": "Alt_L"}, {"keyUp": "Super_L"}, {"wait": 1100},
            # Drag the snip aside, over the editor.
            {"glide": {"x": 580, "y": 520, "ms": 600}}, {"wait": 150},
            {"button": {"press": True}}, {"glide": {"x": 1225, "y": 395, "ms": 900}}, {"wait": 100},
            {"button": {"press": False}}, {"wait": 600},
            # The editor comes forward over the viewer; the snip stays on top.
            # Clicked where neither the viewer (x < 860) nor the snip covers it.
            {"glide": {"x": 1000, "y": 650, "ms": 700}}, {"wait": 150},
            {"button": {"press": True}}, {"wait": 60}, {"button": {"press": False}}, {"wait": 700},
            {"key": "End"}, {"type": typed}, {"wait": 1800},
            {"stopRecord": True}, {"windows": True}]
    xkb = "tr" if lang == "tr" else "us"
    locale = "tr_TR.UTF-8" if lang == "tr" else "en_US.UTF-8"
    scheme = "prefer-dark" if theme == "dark" else "prefer-light"
    log = run(plan, RECORD=1, APP_ARGS="--autostart", XKB=xkb, BG=DESKTOP[theme],
              DEMO=f"export ADW_DEBUG_COLOR_SCHEME={scheme} LANG={locale} LC_ALL={locale}; "
                   f"loupe '{chart}' & gnome-text-editor --standalone '{report}'")
    if "SnipdeckFloating" not in log:
        raise SystemExit("ontop: no floating snip appeared — see shell.log")
    if "dismissed \"Add Keyboard Shortcuts\"" in log and "STILL THERE" in log:
        raise SystemExit("ontop: the shortcuts dialog stayed open")
    encode(shot("ontop.mp4"), os.path.join(out, f"ontop-{lang}-{theme}"), trim=1.4, fade=DESKTOP[theme])


def encode(src, base, width=1600, trim=0.0, fade=None):
    """The web video: AV1 in WebM, an H.264 MP4 for Safari, and a poster
    (the last frame: the snip above the editor). `trim` drops the idle start;
    `fade` (a colour) fades in from it and out to it, so the loop restarts
    without a jump."""
    dur = float(subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", src],
                               capture_output=True, text=True, check=True).stdout) - trim
    vf = f"scale={width}:-2:flags=lanczos,fps=30"
    if fade:
        c = "0x" + fade.lstrip("#")
        vf += f",fade=t=in:st=0:d=0.35:color={c},fade=t=out:st={dur - 0.45:.2f}:d=0.45:color={c}"
    ff = ["ffmpeg", "-v", "error", "-y", "-ss", f"{trim}", "-i", src]
    subprocess.run(ff + ["-vf", vf, "-an", "-c:v", "libsvtav1", "-crf", "38", "-preset", "6",
                         "-pix_fmt", "yuv420p", base + ".webm"], check=True)
    subprocess.run(ff + ["-vf", vf, "-an", "-c:v", "libx264", "-crf", "27", "-preset", "slow",
                         "-pix_fmt", "yuv420p", "-movflags", "+faststart", base + ".mp4"], check=True)
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-sseof", "-0.5", "-i", src, "-frames:v", "1",
                    "-vf", f"scale={width}:-2:flags=lanczos", "-quality", "90", base + ".webp"], check=True)


SCENES = {"gallery": scene_gallery, "floating": scene_floating, "annotate": scene_annotate,
          "collage": scene_collage, "upload": scene_upload, "ontop": scene_ontop}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--langs", default="tr,en")
    ap.add_argument("--themes", default="dark,light")
    ap.add_argument("--scenes", default=",".join(SCENES))
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    if not os.path.isdir(SEEDS):
        subprocess.run([sys.executable, os.path.join(HERE, "seeds.py"), SEEDS, str(SCALE)], check=True)
    for lang in a.langs.split(","):
        settings(lang, "dark")
        seed(lang)
        for theme in a.themes.split(","):
            settings(lang, theme)
            for name in a.scenes.split(","):
                print(f"{name} {lang} {theme}", flush=True)
                restore()
                SCENES[name](lang, theme, a.out)


if __name__ == "__main__":
    main()
