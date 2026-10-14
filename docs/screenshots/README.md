# Screenshots

The root `README.md` and its translations show these four:

| File | What it shows |
| --- | --- |
| `gallery.png` | The gallery: toolbar, snip cards, status bar |
| `floating.png` | Floating snips on the desktop, one with its right-click menu open |
| `annotate.png` | The annotation editor with a box, an arrow, a highlight and a note |
| `collage.png` | The collage editor: five snips laid out, the tray below |

They are not taken by hand. `tools/ui-harness/shots.py` builds them in a
headless GNOME session, at 2x, from made-up sample snips, in Turkish and
English and in both themes (the same run makes the website's images):

```bash
python3 tools/ui-harness/shots.py OUT_DIR
```

These are the English, dark-theme ones (`<scene>-en-dark.png`).
