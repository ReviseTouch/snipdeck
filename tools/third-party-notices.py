#!/usr/bin/env python3
"""Write THIRD-PARTY-NOTICES.md: the licences of everything linked into Snipdeck.

    python3 tools/third-party-notices.py

Snipdeck is one statically linked binary, so the notices its dependencies ask
to travel with binary copies have to be shipped beside it. This lists every
crate that ends up in the Windows or Linux build (normal dependencies only —
build scripts and proc-macros run on the build machine and are not shipped),
with the licence text found in each crate's own source, identical texts
printed once. Run it again after `cargo update` or a dependency change.

Needs the crates in cargo's registry for both targets; `cargo fetch` puts
them there.
"""
import hashlib
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
TARGETS = ["x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
LICENCE_FILE = re.compile(r"^(licen[sc]e|copying|notice|unlicense|copyright)", re.I)

# What is in the binary without being a crate of its own.
BUNDLED = """\
## Bundled with crates

- **libwebp** (in `libwebp-sys`) — Copyright 2010 Google Inc., BSD 3-Clause.
  Its text is below, under `libwebp-sys`.
- **SQLite** (in `libsqlite3-sys`) — public domain.
- **Icons** (`ui/icons.slint`) — line glyphs drawn after
  [Feather](https://feathericons.com) (MIT, Copyright (c) 2013-2023 Cole Bemis)
  and [Lucide](https://lucide.dev) (ISC, Copyright (c) for portions of Lucide
  are held by Cole Bemis 2013-2022 as part of Feather (MIT); all other
  copyright (c) for Lucide are held by Lucide Contributors 2022). Both licence
  texts are at the end of this file.
- **Slint** — the UI toolkit, used under the Slint Royalty-free Licence 2.0;
  the About window carries its "Made with Slint" mark.

Tesseract, GTK, PipeWire and the other system libraries are not part of the
binary: they are separate programs and shared libraries on your system.
"""

ICON_LICENCES = """\
## Feather — MIT

```
The MIT License (MIT)

Copyright (c) 2013-2023 Cole Bemis

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Lucide — ISC

```
ISC License

Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part
of Feather (MIT). All other copyright (c) for Lucide are held by Lucide
Contributors 2022.

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THIS SOFTWARE.
```
"""


def run(*args):
    return subprocess.run(args, cwd=ROOT, check=True, capture_output=True, text=True).stdout


def shipped():
    """`name version` of every crate linked into either target's binary."""
    out = set()
    for target in TARGETS:
        tree = run("cargo", "tree", "--locked", "-e", "normal", "--prefix", "none",
                   "--target", target, "-f", "{p}")
        for line in tree.splitlines():
            parts = line.split()
            if len(parts) >= 2:
                out.add((parts[0], parts[1].lstrip("v")))
    return out


def licence_texts(directory):
    """The licence files at a crate's top level (libwebp's is one down)."""
    found = []
    for folder in (directory, directory / "vendor"):
        if not folder.is_dir():
            continue
        for path in sorted(folder.iterdir()):
            if path.is_file() and LICENCE_FILE.match(path.name) and path.stat().st_size < 200_000:
                text = path.read_text(encoding="utf-8", errors="replace")
                # Line endings and trailing blanks differ between copies of
                # the same text; they should not make it print twice.
                text = "\n".join(line.rstrip() for line in text.replace("\r\n", "\n").split("\n")).strip()
                if text:
                    found.append(text)
    return found


def main():
    meta = json.loads(run("cargo", "metadata", "--locked", "--format-version", "1"))
    wanted = shipped()
    crates = []
    for pkg in meta["packages"]:
        if (pkg["name"], pkg["version"]) in wanted and pkg["name"] != "snipdeck":
            crates.append(pkg)
    crates.sort(key=lambda p: (p["name"].lower(), p["version"]))

    texts = {}      # digest -> text
    users = {}      # digest -> ["name version", ...]
    missing = []
    for pkg in crates:
        label = f'{pkg["name"]} {pkg["version"]}'
        found = licence_texts(pathlib.Path(pkg["manifest_path"]).parent)
        if not found:
            missing.append(label)
        for text in found:
            digest = hashlib.sha256(text.encode()).hexdigest()
            texts[digest] = text
            users.setdefault(digest, []).append(label)

    out = [
        "# Third-party notices",
        "",
        "Snipdeck itself is MIT-licensed (see `LICENSE`). Its binary also contains the",
        "software listed here, each under its own licence. This file is generated by",
        "`tools/third-party-notices.py`; do not edit it by hand.",
        "",
        BUNDLED,
        "## Crates",
        "",
        "| Crate | Version | Licence |",
        "| --- | --- | --- |",
    ]
    for pkg in crates:
        name = pkg["name"]
        link = pkg.get("repository") or f"https://crates.io/crates/{name}"
        out.append(f'| [{name}]({link}) | {pkg["version"]} | {pkg.get("license") or "see its source"} |')
    out += ["", "## Licence texts", ""]
    # Most-shared texts first: Apache-2.0 once, then the long tail.
    for digest in sorted(texts, key=lambda d: (-len(users[d]), users[d][0].lower())):
        out.append("Used by: " + ", ".join(users[digest]))
        out += ["", "```", texts[digest].replace("```", "'''"), "```", ""]
    if missing:
        out += [
            "## Crates that ship no licence file",
            "",
            "These declare the licence in the table above and carry no text of their own:",
            "",
            ", ".join(missing) + ".",
            "",
        ]
    out.append(ICON_LICENCES)

    (ROOT / "THIRD-PARTY-NOTICES.md").write_text("\n".join(out), encoding="utf-8")
    print(f"{len(crates)} crates, {len(texts)} distinct licence texts, {len(missing)} without a file",
          file=sys.stderr)


if __name__ == "__main__":
    main()
