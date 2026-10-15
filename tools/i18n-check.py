#!/usr/bin/env python3
"""Check the translations against the code.

    tools/i18n-check.py [repo]

Every string the UI (`@tr` in ui/*.slint) and the Rust code (`i18n::t`,
`t1`) asks for must be in lang/snipdeck.pot and translated in every
lang/<code>/LC_MESSAGES/snipdeck.po: present, not empty, not fuzzy, and with
the same {placeholders} as the English. English needs no entries (the msgid
is the English). Exits 1 on any problem; CI runs it.
"""
import re, glob, os, sys
root = sys.argv[1] if len(sys.argv) > 1 else "."
os.chdir(root)

def unq(s):
    return bytes(s, "utf-8").decode("unicode_escape").encode("latin-1").decode("utf-8")

def parse_po(path):
    entries = {}; cur = None; field = None; fuzzy = False; nextfuzzy = False
    msgid = msgstr = None
    def flush():
        if msgid is not None:
            entries[msgid] = (msgstr, fuzzy)
    for line in open(path, encoding="utf-8"):
        line = line.rstrip("\n")
        if line.startswith("#,") and "fuzzy" in line:
            nextfuzzy = True; continue
        if line.startswith("#") or not line.strip():
            continue
        m = re.match(r'(msgctxt|msgid|msgstr)\s+"(.*)"$', line)
        if m:
            if m.group(1) in ("msgid", "msgctxt") and field == "msgstr":
                flush(); msgid = msgstr = None
            if m.group(1) == "msgid":
                msgid = unq(m.group(2)); field = "msgid"; fuzzy = nextfuzzy; nextfuzzy = False
            elif m.group(1) == "msgstr":
                msgstr = unq(m.group(2)); field = "msgstr"
            else:
                field = "msgctxt"
            continue
        m = re.match(r'"(.*)"$', line)
        if m:
            if field == "msgid": msgid += unq(m.group(1))
            elif field == "msgstr": msgstr += unq(m.group(1))
    flush()
    entries.pop("", None)
    return entries

# Strings the code asks for
code = set()
for f in glob.glob("ui/**/*.slint", recursive=True):
    for m in re.finditer(r'@tr\(\s*"((?:[^"\\]|\\.)*)"', open(f, encoding="utf-8").read()):
        code.add(unq(m.group(1)))
for f in glob.glob("src/**/*.rs", recursive=True):
    t = open(f, encoding="utf-8").read()
    for m in re.finditer(r'i18n::t1?\(\s*"((?:[^"\\]|\\.)*)"', t):
        code.add(unq(m.group(1)))
    for m in re.finditer(r'(?<![\w:])t1?\(\s*"((?:[^"\\]|\\.)*)"', t) if re.search(r"use crate::i18n::\{?[^;]*\bt1?\b", t) else []:
        code.add(unq(m.group(1)))

# Built at run time from a list (src/app/mod.rs short_date).
code |= {"Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"}
pot = parse_po("lang/snipdeck.pot")
print(f"code strings: {len(code)}   pot entries: {len(pot)}")
missing_pot = sorted(code - set(pot))
stale_pot = sorted(set(pot) - code)
print(f"in code, not in pot: {len(missing_pot)}")
for s in missing_pot: print("   +", repr(s)[:110])
print(f"in pot, not in code: {len(stale_pot)}")
for s in stale_pot: print("   -", repr(s)[:110])

ph = re.compile(r'\{[a-z_0-9]*\}')
total_bad = 0
total_bad += len(missing_pot) + len(stale_pot)
for po in sorted(glob.glob("lang/*/LC_MESSAGES/*.po")):
    lang = po.split("/")[1]
    if lang == "en":
        continue
    e = parse_po(po)
    need = code
    miss = [s for s in need if s not in e]
    empty = [s for s in need if s in e and not e[s][0]]
    fuzzy = [s for s in need if s in e and e[s][1]]
    badph = [s for s in need if s in e and e[s][0] and sorted(ph.findall(s)) != sorted(ph.findall(e[s][0]))]
    same = [s for s in need if s in e and e[s][0] == s and len(s) > 12]
    dup = len(e)
    bad = len(miss) + len(empty) + len(fuzzy) + len(badph)
    total_bad += bad
    print(f"{lang:6} entries={dup:4} missing={len(miss):3} empty={len(empty):3} fuzzy={len(fuzzy):3} placeholder={len(badph):3} untranslated-looking={len(same):3}")
    for s in (miss + empty)[:6]: print("      ·", repr(s)[:100])
    for s in badph[:6]: print("      ph", repr(s)[:60], "->", repr(e[s][0])[:60])
    for s in same[:4]: print("      =", repr(s)[:80])
print("problems:", total_bad)
sys.exit(1 if total_bad else 0)
