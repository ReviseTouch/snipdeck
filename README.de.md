<div align="center">

[English](README.md) · [Türkçe](README.tr.md) · [Español](README.es.md) · [简体中文](README.zh-CN.md) · [العربية](README.ar.md) · [Русский](README.ru.md) · **Deutsch** · [日本語](README.ja.md) · [Français](README.fr.md) · [Português](README.pt.md)

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**Ein schnelles, tastaturgesteuertes _Deck_ für Bildschirmausschnitte unter Windows.**

Erfasse einen Bereich, halte Ausschnitte über allem schwebend, versieh sie mit Anmerkungen,
indexiere sie per OCR und durchsuche hunderte Screenshots aus einer einzigen, dichten Galerie –
alles aus einer nativen Rust-App mit einer softwaregerenderten [Slint](https://slint.dev)-Oberfläche.

![platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows&logoColor=white)
![soon](https://img.shields.io/badge/macOS%20%2F%20Linux-on%20the%20way-lightgrey)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Slint-orange?logo=rust&logoColor=white)
![languages](https://img.shields.io/badge/languages-23-success)
![license](https://img.shields.io/badge/license-MIT-green)

</div>

---

## 📸 Screenshots

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · translate.png
-->

<div align="center">

| Der Galerie-Arbeitsbereich | Schwebende Ausschnitte auf dem Desktop |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="Galerie" width="420" /> | <img src="docs/screenshots/floating.png" alt="Schwebende Ausschnitte" width="420" /> |
| **Der Anmerkungseditor** | **OCR + Übersetzen** |
| <img src="docs/screenshots/annotate.png" alt="Anmerkungseditor" width="420" /> | <img src="docs/screenshots/translate.png" alt="OCR-Übersetzung" width="420" /> |

</div>

---

## ✨ Highlights

- 🖱️ **Erfassen per Modifikatortaste + Ziehen.** Halte `Win` und ziehe einen Bereich auf; die Modifikatoren entscheiden, was
  mit dem Ergebnis geschieht – schwebendes Fenster, Zwischenablage, nur Galerie (Matrix unten).
- 🧊 **Erfassen mit Einfrieren zuerst** (`Win`+`Shift`+`Space`). Dimmt und friert den Bildschirm ein, *bevor* du
  auswählst, sodass Hover-Tooltips und andere klick-empfindliche UI-Elemente erfasst bleiben.
- 📌 **Schwebende Ausschnitte.** Erfassungen bleiben als leichtgewichtige, rahmenlose Overlays oben angeheftet, die du
  ziehen, skalieren, anheften, zuschneiden, drehen/spiegeln und schließen kannst.
- ✏️ **Integrierter Anmerkungseditor.** Stift, Textmarker, Formen, Pfeile und Text mit einem vollständigen HSV-
  Farbkreis, einstellbarer Größe + Deckkraft, Zoom sowie Rückgängig/Wiederholen (Details).
- 🔍 **Integrierte OCR.** Jeder Ausschnitt wird im Hintergrund mit der Betriebssystem-Engine OCR-indexiert
  (`Windows.Media.Ocr`) – kein Modell-Download, kein Netzwerk, vollständig lokal.
- 🌐 **OCR + Übersetzen.** Lies den Text eines Ausschnitts inline in einer anderen Sprache (30+ Zielsprachen, mit
  Favoriten, die du markieren kannst).
- 🗂️ **Leistungsstarke Galerie.** Volltextsuche über OCR-Text, Fenster-Metadaten, Monitor und Pfad;
  Filtern nach Status / Inhalt / Größe / Datum; ein virtualisiertes Raster, das auch bei tausenden Ausschnitten flüssig bleibt.
- 📤 **Teilen & Hochladen.** Windows-Teilen-Dialog, eine neue E-Mail (Simple MAPI), der System-Bildeditor
  oder ein Bild-Hoster – Uploads sind **opt-in mit einem expliziten Einwilligungsdialog**.
- 🌍 **23 Sprachen**, automatisch aus deinem Betriebssystem erkannt und über die Symbolleiste umschaltbar.
- 🌗 **Helle / dunkle Designs**, ein fester Platz im System-Tray und **registry-freier** Start bei der Anmeldung.

---

## ⌨️ Tastenkürzel zum Erfassen

Halte `Win` und ziehe dann mit der **linken** Maustaste einen Bereich auf. Füge Modifikatoren hinzu, um zu ändern, was
mit dem Ausschnitt geschieht:

| Tastenkürzel | Ergebnis |
| --- | --- |
| `Win` + Ziehen | Schwebender Ausschnitt auf dem Bildschirm |
| `Win`+`Ctrl` + Ziehen | Schwebender Ausschnitt **und** Kopie in die Zwischenablage |
| `Win`+`Alt` + Ziehen | Nur Zwischenablage (nicht zur Galerie hinzugefügt) |
| `Win`+`Shift` + Ziehen | Nur Galerie (kein schwebendes Fenster) |
| `Win`+`Ctrl`+`Shift` + Ziehen | Galerie **und** Zwischenablage, kein schwebendes Fenster |
| `Win`+`Shift`+`Space`, dann Ziehen | **Einfrieren zuerst** – friere den Bildschirm ein und wähle dann auf dem Standbild aus |

Du kannst auch mit der rechten Maustaste auf das Tray-Symbol klicken → **Neuer Ausschnitt**, um eine einmalige Zieherfassung zu aktivieren, ohne
`Win` zu halten.

### Globale Tastenkürzel

Neben `Win` + Ziehen registriert Snipdeck fünf konfigurierbare globale Tastenkürzel:

| Tastenkürzel | Aktion |
| --- | --- |
| `Win`+`Alt`+`H` | Alle Ausschnitte auf dem Bildschirm ein- / ausblenden |
| `Ctrl`+`Shift`+`A` | Neuer Ausschnitt |
| `Ctrl`+`Shift`+`C` | Neuer Ausschnitt **und** in die Zwischenablage kopieren |
| `Ctrl`+`Shift`+`X` | Nur Zwischenablage |
| `Ctrl`+`Shift`+`G` | Galerie öffnen |

Ändere sie über die `hotkey_*`-Schlüssel in `settings.json`.

---

## ✏️ Anmerkungseditor

Klicke mit der rechten Maustaste auf einen beliebigen Ausschnitt → **Anmerken**, um einen fokussierten Editor im Telegram-Stil zu öffnen, der über
deinen Ausschnitten liegt.

- **Werkzeuge** – Freihandstift, Textmarker, Rechteck, Ellipse, Linie, Pfeil und Text.
- **Vollständiger HSV-Farbkreis** mit einem Helligkeitsregler – wähle *jede* Farbe, nicht nur eine feste Palette.
- **Regler für Größe & Deckkraft** mit einer blinkenden Größenvorschau auf dem Bild, pro Werkzeug gespeichert.
- **Hinein- / Heraus- / Einpassen-Zoom** für präzises Arbeiten; die Leinwand verschiebt sich beim Hineinzoomen.
- **Rückgängig / Wiederholen / Löschen**, dazu `Esc`, `Ctrl`+`Z`, `Ctrl`+`Y`.
- Beim **Speichern** werden die Striche in voller Auflösung fest in das Bild eingebrannt (scharf, in nativer Größe).

Die Live-Vorschau und das gespeicherte Ergebnis stammen aus demselben Code, sodass **was du siehst, auch das ist, was du bekommst**.

---

## 🖱️ Rechtsklick-Aktionen

Klicke mit der rechten Maustaste auf einen schwebenden Ausschnitt **oder** ein Galerie-Miniaturbild:

- **Kopieren** des Bildes in die Zwischenablage
- **OCR + Zwischenablage** – OCR ausführen und den erkannten Text kopieren
- **OCR + Übersetzen** – das Übersetzungs-Popup öffnen
- **Anmerken** – den oben beschriebenen Editor öffnen
- **Teilen** – Windows-Teilen-Dialog, **Mail** (MAPI-Anhang), in **Editor** öffnen (Paint)
  oder zu einem Bild-Hoster **Hochladen**
- **Transformieren** – drehen (90° / 180°), horizontal / vertikal spiegeln
- **Zuschneiden** (die Griffe nach innen ziehen) und **Größe ändern**
- **Vom Bildschirm ausblenden** / **Löschen**

---

## 📦 Installation

### Herunterladen

Hol dir die neueste `snipdeck.exe` von der Seite [**Releases**](https://github.com/hasantr/snipdeck/releases).
Es handelt sich um eine einzelne portable ausführbare Datei – kein Installer.

### Aus dem Quellcode erstellen

Voraussetzungen: [Rust](https://rustup.rs/) (stable) und Windows 10 oder 11.

```powershell
git clone https://github.com/hasantr/snipdeck.git
cd snipdeck
cargo build --release
```

Die Binärdatei landet unter `target\release\snipdeck.exe`.

---

## 🌍 Sprachen

Snipdeck ist vollständig lokalisiert. Beim ersten Start wählt es deine Sprache anhand der OS-Locale;
überschreibe sie jederzeit über das **Globus-Menü** in der Symbolleiste.

> Englisch · Spanisch · Vereinfachtes Chinesisch · Hindi · Arabisch · Portugiesisch · Russisch · Japanisch ·
> Deutsch · Französisch · Koreanisch · Italienisch · Türkisch · Indonesisch · Vietnamesisch · Polnisch · Ukrainisch ·
> Niederländisch · Persisch · Thai · Bengali · Urdu · Rumänisch

Die gesamte App – sowohl die Slint-Oberfläche (`@tr`) als auch die Meldungen auf der Rust-Seite (`i18n::t`) – liest aus
**einer gettext-`.po`-Datei pro Sprache** unter `lang/<code>/LC_MESSAGES/snipdeck.po`, die zur Build-Zeit in die
Binärdatei eingebettet wird (keine Laufzeitabhängigkeit von gettext).

<details>
<summary><b>Eine Übersetzung hinzufügen oder aktualisieren</b></summary>

1. Neue Zeichenketten werden in `@tr("…")` (in `.slint`) oder `i18n::t("…")` (in Rust) eingeschlossen.
2. `lang/snipdeck.pot` ist die Vorlage, die jede englische Zeichenkette auflistet.
3. Kopiere eine neue msgid in jede `lang/<code>/LC_MESSAGES/snipdeck.po` und fülle ihr `msgstr` aus –
   halte Platzhalter (`{}`, `{e}`, `{url}`, `{path}`) und `\r\n`-Escapes intakt.
4. Um eine neue Sprache hinzuzufügen, erstelle ihre `.po` und füge den Code zu `LANGUAGES` und `po_source()`
   in `src/i18n.rs` hinzu.

</details>

---

## 🔒 Datenschutz & Sicherheit

- **Erfassung und OCR sind vollständig lokal.** OCR nutzt die integrierte `Windows.Media.Ocr`-Engine; nichts
  wird für Erfassung oder Indexierung irgendwohin gesendet.
- **Der Start bei der Anmeldung ist registry-frei.** Snipdeck legt eine normale Verknüpfung in deinem `shell:Startup`-
  Ordner ab, anstatt einen `HKCU\…\Run`-Wert zu schreiben – der gewöhnliche, für den Nutzer sichtbare Mechanismus, der
  den Autostart-Registry-Schlüssel vermeidet, den Antiviren- / EDR-Tools am häufigsten kennzeichnen.
- **Uploads sind opt-in.** Nichts verlässt deinen Rechner ohne einen expliziten Einwilligungsdialog, der
  das Ziel klar benennt. Lade keine sensiblen Inhalte zu einem per Link öffentlich zugänglichen Hoster hoch.
- **OCR + Übersetzen nutzt das Netzwerk.** Nur der per OCR erkannte *Text* wird an Googles öffentlichen
  `translate_a`-Endpunkt zur Übersetzung gesendet; er wird sonst nirgendwo verwendet.

---

## 🗺️ Roadmap

- [x] Windows 10 / 11
- [ ] **macOS** – in Arbeit
- [ ] **Linux** – in Arbeit
- [ ] In-App-Einstellungsfenster
- [ ] Mehr Anmerkungswerkzeuge (Weichzeichnen / Verpixeln, nummerierte Schritt-Badges, Formen verschieben/bearbeiten)

> Snipdeck ist heute ausschließlich für Windows, da Erfassung, OCR, der Tray und das Teilen allesamt
> Windows-spezifische APIs verwenden. Die Oberfläche ([Slint](https://slint.dev)) und der Großteil des Kerns sind bereits
> plattformübergreifend, sodass **macOS- und Linux-Builds in Arbeit sind.**

---

## ⚙️ Konfiguration

Einstellungen werden als JSON gespeichert und beim ersten Start erstellt. Es gibt noch kein Einstellungsfenster – bearbeite die
Datei direkt und starte Snipdeck dann neu.

| Pfad | Inhalt |
| --- | --- |
| `%APPDATA%\Snipdeck\settings.json` | Benutzereinstellungen (unten) |
| `%LOCALAPPDATA%\Snipdeck\` | `snips.db`, `state.json` und der `cache\` mit Bildern + Miniaturbildern |
| `Pictures\Snipdeck\` | Standardordner für *Speichern* |

<details>
<summary><b>Ausgewählte <code>settings.json</code>-Schlüssel</b></summary>

| Schlüssel | Standard | Bedeutung |
| --- | --- | --- |
| `language` | `"auto"` | UI-Sprachcode oder `"auto"`, um dem Betriebssystem zu folgen |
| `dark_theme` | `true` | Helle oder dunkle Oberfläche |
| `upload_service` | `"catbox"` | Bild-Hoster für *Hochladen*: `"catbox"` (kein Schlüssel) oder `"imgur"` |
| `imgur_client_id` | `""` | Imgur-Client-ID, nur erforderlich, wenn `upload_service` `"imgur"` ist |
| `translate_target` | `"en"` | Standard-Zielsprachcode für die OCR-Übersetzung |
| `translate_favs` | `["en","es","fr","de","it","pt","ru","ar"]` | Schnellauswahl-Sprachen im Übersetzungs-Popup |
| `default_format` | `"png"` | Standard-Dateiendung für *Speichern* |

**Upload-Dienste** – *catbox.moe* (Standard) benötigt kein Konto und keinen Schlüssel, aber anonyme Uploads
**können von dir nachträglich nicht gelöscht werden**. *Imgur* benötigt eine kostenlose Client-ID (einmal registrieren unter
imgur.com → Settings → Applications). In beiden Fällen ist das Bild nicht gelistet, aber jeder mit dem
Link kann es öffnen, und es ist nicht verschlüsselt.

</details>

---

## 🧩 Architektur

Eine single-threaded Slint-Ereignisschleife steuert die Oberfläche; OS-Hooks und blockierende Arbeit laufen auf ihren eigenen
Threads und reichen Ergebnisse über Kanäle zurück, die einmal pro Tick geleert werden.

<details>
<summary><b>Modulkarte</b></summary>

| Modul | Verantwortlichkeit |
| --- | --- |
| `src/app/` | App-Zustand, die Pro-Tick-Schleife, UI-Aktionen, Galerie-Modell |
| `src/capture.rs`, `src/dxgi_capture.rs` | Monitor-Erfassung (DXGI Desktop Duplication, GDI-Fallback) |
| `src/mouse_trigger.rs`, `src/native_mouse_trigger.rs` | Low-Level-Maus-/Tastatur-Hooks, die Ausschnitte aktivieren und steuern |
| `src/slint_overlay.rs`, `ui/overlay.slint` | Vollbild-Auswahl-Overlay (live oder eingefroren) |
| `src/floating.rs`, `ui/floating.slint` | Schwebende Ausschnittfenster (ziehen, skalieren, zuschneiden, Rahmen) |
| `src/annotate.rs`, `ui/annotate.slint` | Anmerkungseditor (tiny-skia-Flatten, HSV-Kreis, Zoom) |
| `src/ocr.rs` | `Windows.Media.Ocr`-Wrapper (auf einem Worker-Thread) |
| `src/translate.rs`, `src/translate_popup.rs` | OCR-Übersetzungs-Popup |
| `src/share.rs`, `src/upload.rs` | Teilen-Dialog / MAPI-Mail / Editor / Bild-Hoster-Upload |
| `src/tray.rs`, `src/autostart.rs`, `src/single_instance.rs` | Tray-Symbol, Start bei der Anmeldung, Single-Instance-Schutz |
| `src/snip.rs`, `src/settings.rs`, `src/paths.rs` | Galerie-Persistenz (SQLite + FTS), Einstellungen, Pfade |
| `src/i18n.rs`, `lang/` | Laufzeit-i18n + eingebettete gettext-`.po`-Kataloge (23 Sprachen) |

</details>

---

## Lizenz

[MIT](LICENSE) © 2026 Hasan Salihoğlu

<div align="center">
<sub>Erstellt mit 🦀 Rust und <a href="https://slint.dev">Slint</a>.</sub>
</div>
