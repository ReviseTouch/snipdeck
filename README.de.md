<div align="center">

[English](README.md) · [Türkçe](README.tr.md) · [Español](README.es.md) · [简体中文](README.zh-CN.md) · [العربية](README.ar.md) · [Русский](README.ru.md) · **Deutsch** · [日本語](README.ja.md) · [Français](README.fr.md) · [Português](README.pt.md)

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**Ein schnelles, tastaturgesteuertes _Deck_ für Bildschirmausschnitte unter Windows und Linux.**

<sub>Eine kostenlose Open-Source-App von <a href="https://revisetouch.com"><b>ReviseTouch</b></a>.</sub>

Erfasse einen Bereich, halte Ausschnitte über allem schwebend, versieh sie mit Anmerkungen,
indexiere sie per OCR und durchsuche hunderte Screenshots aus einer einzigen, dichten Galerie –
alles aus einer nativen Rust-App mit einer [Slint](https://slint.dev)-Oberfläche.

![platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows&logoColor=white) ![release](https://img.shields.io/github/v/release/ReviseTouch/snipdeck?label=release)
![linux](https://img.shields.io/badge/Linux-GNOME%20%C2%B7%20KDE%20%C2%B7%20X11%20%28beta%29-FCC624?logo=linux&logoColor=black) ![soon](https://img.shields.io/badge/macOS-on%20the%20way-lightgrey)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Slint-orange?logo=rust&logoColor=white)
![languages](https://img.shields.io/badge/languages-23-success)
![license](https://img.shields.io/badge/license-MIT-green)

<br/>
<picture>
  <source media="(prefers-color-scheme: light)" srcset="docs/screenshots/hero-en-light.png">
  <img src="docs/screenshots/hero-en-dark.png" alt="The Snipdeck gallery, with two snips floating on the desktop" width="100%">
</picture>

</div>

---

## 📸 Screenshots

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · collage.png
-->

<div align="center">

| Der Galerie-Arbeitsbereich | Schwebende Ausschnitte auf dem Desktop |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="Galerie" width="420" /> | <img src="docs/screenshots/floating.png" alt="Schwebende Ausschnitte" width="420" /> |
| **Der Anmerkungseditor** | **Der Collage-Editor** |
| <img src="docs/screenshots/annotate.png" alt="Anmerkungseditor" width="420" /> | <img src="docs/screenshots/collage.png" alt="Collage-Editor" width="420" /> |

</div>

---

## ✨ Highlights

- 🖱️ **Erfassen per Modifikatortaste + Ziehen.** Halte `Win` und ziehe einen Bereich auf; die Modifikatoren entscheiden, was
  mit dem Ergebnis geschieht – schwebendes Fenster, Zwischenablage, nur Galerie ([Matrix unten](#️-tastenkürzel-zum-erfassen)).
- 🧊 **Erfassen mit Einfrieren zuerst** (`Win`+`Shift`+`Space`). Dimmt und friert den Bildschirm ein, *bevor* du
  auswählst, sodass Hover-Tooltips und andere klick-empfindliche UI-Elemente erfasst bleiben.
- 📌 **Schwebende Ausschnitte.** Erfassungen bleiben als leichtgewichtige, rahmenlose Overlays oben angeheftet, die du
  ziehen, skalieren, anheften, zuschneiden, drehen/spiegeln und schließen kannst.
- ✏️ **Integrierter Anmerkungseditor.** Stift, Textmarker, Formen, Pfeile und Text mit einem vollständigen HSV-
  Farbkreis, einstellbarer Größe + Deckkraft, Zoom sowie Rückgängig/Wiederholen ([Details](#️-anmerkungseditor)).
- 🧩 **Collage.** Lege mehrere Ausschnitte zu einem Bild zusammen – Raster, ausgerichtete Zeilen oder Masonry, mit
  Abstand, abgerundeten Ecken und Hintergrund –, das als neuer Ausschnitt zurück in die Galerie gespeichert wird.
- 🔍 **Integrierte OCR.** Jeder Ausschnitt wird im Hintergrund mit der Betriebssystem-Engine OCR-indexiert
  (`Windows.Media.Ocr`) – kein Modell-Download, kein Netzwerk, vollständig lokal.
- 🌐 **OCR + Übersetzen.** Lies den Text eines Ausschnitts inline in einer anderen Sprache (29 Zielsprachen, mit
  Favoriten, die du markieren kannst).
- 🗂️ **Leistungsstarke Galerie.** Volltextsuche über OCR-Text, Fenster-Metadaten, Monitor und Pfad;
  Filtern nach Status / Inhalt / Größe / Datum; ein virtualisiertes Raster, das auch bei tausenden Ausschnitten flüssig bleibt
  ([Rundgang](#️-die-galerie)).
- 📤 **Teilen & Hochladen.** Windows-Teilen-Dialog, eine neue E-Mail (Simple MAPI), der System-Bildeditor
  oder ein Bild-Hoster – catbox.moe, Imgur oder **dein eigener Server** aus einer ShareX-`.sxcu`-Datei
  ([Details](#-dein-eigener-bild-hoster)). Jeder Upload **fragt zuerst**.
- 🌍 **23 Sprachen**, automatisch aus deinem Betriebssystem erkannt und über das `⋯`-Menü der Symbolleiste umschaltbar.
- 🪶 **Schont deinen Rechner.** Nativer Code in Rust mit einer [Slint](https://slint.dev)-Oberfläche – keine Browser-Engine, keine Laufzeitumgebung zum Installieren. Rund 68 MB Arbeitsspeicher und kaum CPU-Last, während es im Tray wartet ([gemessen](#-ressourcenverbrauch)).
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
| `Ctrl`+`Alt`+`Shift`+`A` | Neuer Ausschnitt |
| `Ctrl`+`Alt`+`Shift`+`C` | Neuer Ausschnitt **und** in die Zwischenablage kopieren |
| `Ctrl`+`Alt`+`Shift`+`X` | Nur Zwischenablage |
| `Ctrl`+`Alt`+`Shift`+`G` | Galerie öffnen |

Ändere sie über die `hotkey_*`-Schlüssel in `settings.json`. Unter Wayland gehören globale
Tastenkürzel dem Desktop: Sie werden nur beim ersten Start vorgeschlagen, danach änderst du sie
in den Tastatureinstellungen deines Desktops.

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
- **Teilen** – Windows-Teilen-Dialog, **E-Mail** (MAPI-Anhang), in **Editor** öffnen (Paint)
  oder zu einem Bild-Hoster **Hochladen** ([zu welchem](#-dein-eigener-bild-hoster))
- **Transformieren** – drehen (90° / 180°), horizontal / vertikal spiegeln
- **Zuschneiden** (die Griffe nach innen ziehen) und **Größe ändern**
- **Vom Bildschirm ausblenden** / **Löschen**

---

## 🗂️ Die Galerie

Eine einzige Symbolleistenzeile; alles andere ist nur einen Klick entfernt.

- **Neu** erstellt einen Ausschnitt. Sein Pfeil enthält **Ausschnitt + Kopie** und **Nur Kopie**, jeweils mit
  ihrem Tastenkürzel.
- **Collage** und **Farbe wählen** sitzen daneben, und das Suchfeld nimmt den Rest der Zeile ein –
  `Ctrl`+`F` setzt den Fokus hinein, `Esc` leert es.
- **Filtern** öffnet Status, Inhalt, Größe und Datum in einem einzigen Panel. Aktive Filter erscheinen als Chips
  unter der Symbolleiste; jeder lässt sich einzeln entfernen, oder alle auf einmal mit **Filter zurücksetzen**.
- **`⋯`** enthält, was sich selten ändert: Schwebende Ausschnitte in der Taskleiste, Dunkles Design, Beim
  Anmelden starten, Sprache, das Leeren der Galerie und Über Snipdeck.
- Die Statusleiste zählt, was angezeigt wird, und enthält **Sortierung** sowie die Kartengröße **S / M / L**.

Fahre mit der Maus über eine Karte, um ihre Schnellaktionen zu sehen; ein Rechtsklick bietet [alles andere](#️-rechtsklick-aktionen).

---

## 📤 Dein eigener Bild-Hoster

**`⋯` → Hochladen zu** legt fest, wohin **Hochladen** einen Ausschnitt schickt: catbox.moe (Standard, kein Konto),
Imgur oder ein eigener Hoster. Um einen hinzuzufügen, wähle **Aus .sxcu-Datei hinzufügen…** und dann eine
ShareX-Custom-Uploader-Datei – das Format, das Zipline, XBackBone, Chevereto und die meisten selbst gehosteten
Bildserver ohnehin anbieten; du musst also nichts von Hand schreiben.

Snipdeck sendet das Bild als Multipart-Formulardaten oder als binären Body, mit den Headern, Argumenten und
Query-Parametern aus der Datei, und liest den Link mit `{json:…}`, `{regex:…}`, `{header:…}` oder `{response}` aus –
auch in der `$json:…$`-Form von ShareX 12. Eine Datei, die mehr braucht (eine Eingabeaufforderung, einen Zufallswert,
eine XML-Antwort, ein Ziel, das kein Bild-Hoster ist), wird schon beim Import mit Begründung abgelehnt, statt
erst beim ersten Upload zu scheitern.

Bevor irgendetwas gesendet wird, nennt Snipdecks eigenes Fenster den Hoster und dessen Adresse und sagt, was der
Upload bedeutet; danach zeigt es den Link, der bereits kopiert ist.

---

## 📦 Installation

### Herunterladen

**Windows** – hol dir `snipdeck-windows-x64.exe` von der Seite
[**Releases**](https://github.com/ReviseTouch/snipdeck/releases). Eine einzelne
portable ausführbare Datei, kein Installer.

**Linux (x64)** – von derselben Seite das Paket für deine Distribution:

```bash
sudo apt install ./snipdeck_*_amd64.deb      # Debian, Ubuntu, Mint
sudo dnf install ./snipdeck-*.x86_64.rpm     # Fedora, RHEL, Rocky
sudo zypper install ./snipdeck-*.x86_64.rpm  # openSUSE
```

oder, auf jeder Distribution und ohne Root-Rechte, das Tarball-Archiv:

```bash
tar xzf snipdeck-linux-x64.tar.gz
cd snipdeck-linux-x64
./install.sh          # into ~/.local; --uninstall reverses it
```

Die Pakete laufen auf jeder Distribution mit glibc 2.30 oder neuer – Debian 11, Ubuntu
20.04, RHEL 9 und alles Neuere – und wurden auf Debian 12 und 13, Ubuntu 22.04 und 24.04,
Fedora, Rocky 9, openSUSE Leap 15.6 und Tumbleweed installiert und gestartet
(`packaging/linux/test-install.sh`). Unter GNOME und KDE Plasma installieren sie außerdem
Snipdecks Desktop-Helfer (siehe unten), den Snipdeck beim ersten Start einschaltet; GNOME
lädt eine neu installierte Erweiterung bei der nächsten Anmeldung.

### Aus dem Quellcode erstellen

Voraussetzungen: [Rust](https://rustup.rs/) (stable).

```bash
git clone https://github.com/ReviseTouch/snipdeck.git
cd snipdeck
cargo build --release
```

**Windows** (10 oder 11): Die Binärdatei landet unter `target\release\snipdeck.exe`.

**Linux**: Der Build benötigt die Header für GTK, X11/Wayland, Tray, D-Bus und GL.
Unter Debian/Ubuntu:

```bash
sudo apt install libgtk-3-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxcb-randr0-dev libxrandr-dev libxkbcommon-dev \
  libwayland-dev libayatana-appindicator3-dev libfontconfig-dev libdbus-1-dev \
  libgl1-mesa-dev libegl1-mesa-dev
```

PipeWire wird zur Build-Zeit nicht benötigt: Es wird zur Laufzeit für die Wayland-Erfassung
geladen. Um deinen Build so zu installieren, wie ein Release installiert wird, lege ihn neben
den Installer:

```bash
packaging/linux/install-from-source.sh   # builds, stages afresh, installs
```

---

## 🌍 Sprachen

Snipdeck ist vollständig lokalisiert. Beim ersten Start wählt es deine Sprache anhand der OS-Locale;
überschreibe sie jederzeit über **`⋯` → Sprache** in der Symbolleiste.

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
5. Führe `tools/i18n-check.py` aus: Es prüft, ob jeder Text samt Platzhaltern übersetzt ist; die CI führt es ebenfalls aus.

</details>

---

## 🔒 Datenschutz & Sicherheit

- **Erfassung und OCR sind lokal.** OCR nutzt unter Windows `Windows.Media.Ocr` und unter Linux
  Tesseract (über eine Pipe gespeist, ohne temporäre Datei); für Erfassung oder Indexierung wird
  nichts irgendwohin gesendet. Unter Wayland wird der Bildschirm über das ScreenCast-Portal
  gelesen: Du erteilst die Freigabe einmal, und die Freigabeanzeige erscheint nur in dem Moment,
  in dem ein Ausschnitt aufgenommen wird.
- **OCR + Übersetzen fragt zuerst.** Beim ersten Mal erklärt es, dass der per OCR erkannte
  *Text* – niemals das Bild – an Google Übersetzer geht, und wartet auf dein Ja.
  Der Text wird im Request-Body gesendet, nicht in der URL.
- **Uploads fragen zuerst, jedes Mal.** Nichts verlässt deinen Rechner, bevor du **Hochladen** in einem Fenster
  drückst, das den Hoster und seine Adresse nennt und sagt, was das bedeutet: Jeder mit dem Link kann das Bild
  öffnen, ein catbox-Upload lässt sich nicht löschen, ein einfacher `http://`-Server sendet es unverschlüsselt.
  Lade keine sensiblen Inhalte zu einem per Link öffentlich zugänglichen Hoster hoch. API-Schlüssel aus einer
  importierten `.sxcu`-Datei werden in `settings.json` gespeichert, so wie ShareX sie speichert.
- **Übergabekopien bleiben privat.** Bilder, die an ein Mailprogramm oder einen Editor übergeben
  werden, landen in einem Ordner, den nur du lesen kannst (`$XDG_RUNTIME_DIR` unter Linux,
  `%TEMP%\Snipdeck` unter Windows), und werden danach entfernt.
- **Logs hinterlassen keine Spuren.** Zeigerkoordinaten und Fenstertitel werden nur auf
  Debug-Ebene protokolliert.
- **Der Start bei der Anmeldung ist dateibasiert.** Eine normale Verknüpfung in `shell:Startup`
  unter Windows (nicht der `HKCU\…\Run`-Schlüssel, den Sicherheitstools am häufigsten kennzeichnen),
  ein XDG-Autostart-Eintrag unter Linux.

---

## 🗺️ Roadmap

- [x] Windows 10 / 11
- [x] **Linux / GNOME** (Wayland) – mit der mitgelieferten Shell-Erweiterung: Erfassen
      mit `Super`+`Alt`+Ziehen oder den Tastenkürzeln, Ausschnitte erscheinen dort, wo sie
      erfasst wurden, und bleiben im Vordergrund, OCR, die Farbauswahl
- [x] **Linux / KDE Plasma** (Wayland) – mit dem mitgelieferten KWin-Skript: Ausschnitte
      erscheinen dort, wo sie erfasst wurden, und bleiben im Vordergrund; Erfassen über
      die Tastenkürzel und die Symbolleiste, OCR, die Farbauswahl
- [x] **Linux / X11** (Xfce, MATE, Cinnamon, GNOME unter Xorg…) – alles, Ausschnitte
      werden vom Fenstermanager im Vordergrund gehalten
- [~] **Linux / wlroots** (sway, Hyprland, river) – Erfassen über die Tastenkürzel und die
      Symbolleiste, OCR, die Farbauswahl; Ausschnitte werden weder platziert noch im
      Vordergrund gehalten (siehe unten)

- [ ] **macOS** – in Arbeit
- [ ] In-App-Einstellungsfenster
- [ ] Mehr Anmerkungswerkzeuge (Weichzeichnen / Verpixeln, nummerierte Schritt-Badges, Formen verschieben/bearbeiten)

Außerhalb von GNOME gibt es keine Modifikator+Ziehen-Geste: Sie bräuchte einen globalen
Zeiger-Hook, den Linux nur durch Lesen von `/dev/input` gewährt – ein systemweiter
Keylogger, um dessen Aktivierung Snipdeck dich nicht bitten wird.

### Unter Linux ausführen

Globale Tastenkürzel laufen unter Wayland über das Desktop-Portal, das eine Sitzung von
einem Client ohne Anwendungs-ID ablehnt. Über das Anwendungsmenü gestartet, hat Snipdeck
eine (`com.revisetouch.Snipdeck`, der Name seines Desktop-Eintrags); aus einem Terminal
gestartet, registriert es dieselbe ID selbst, was xdg-desktop-portal 1.19 oder neuer
voraussetzt – bei einem älteren Portal nimm das Menü.

Laufzeitabhängigkeiten: `libayatana-appindicator` (Tray), `xdg-utils` (Teilen) und
`tesseract` samt den gewünschten Sprachdaten (OCR – optional; ohne es wird OCR übersprungen,
statt fehlzuschlagen).

**Was Wayland nicht erlaubt.** Ein Wayland-Client kann seine eigenen Fenster weder
positionieren noch abfragen, wo sie sind, noch verlangen, dass sie über anderen bleiben; alle
drei Dinge gehören dem Compositor. Daher öffnet sich ein Ausschnitt in einer reinen
Wayland-Sitzung dort, wo der Compositor ihn hinsetzt, statt über dem Bereich, aus dem er erfasst
wurde; er kann nicht über anderen Anwendungen schweben, und er erscheint in der Fensterliste.
Alles andere – Bereichserfassung, das Einfrier-Overlay, globale Tastenkürzel, OCR, Ziehen, das
Kontextmenü – funktioniert.

**Unter KDE Plasma erledigt ein KWin-Skript dasselbe** (`packaging/kwin-script/`): Es hält
Ausschnitte über anderen Fenstern, platziert sie dort, wo sie erfasst wurden, legt sie auf
jede Arbeitsfläche und hält sie aus der Fensterleiste heraus. Es wird ohne erneute
Anmeldung geladen.

Auf den wlroots-Compositoren (sway, Hyprland, river) leistet das bisher nichts:
Ausschnitte öffnen sich dort, wo der Compositor sie hinsetzt, und bleiben nicht im
Vordergrund. Eine Fensterregel in der Konfiguration des Compositors selbst kann sie
schwebend machen – ihr Titel lautet `SnipdeckFloating …`.

**Unter GNOME schließt eine Shell-Erweiterung die Lücke.** Was ein Client nicht darf, darf
Code, der innerhalb der GNOME Shell läuft. `install.sh` installiert daher zusätzlich
`snipdeck@revisetouch.com` (aus `packaging/gnome-extension/`), wenn es eine GNOME-Sitzung
erkennt – eine einzelne Datei, die auf Snipdecks Ausschnittfenster achtet, sie dort platziert,
wo sie erfasst wurden, sie über anderen Fenstern hält, sie an jede Arbeitsfläche heftet und sie
aus dem Fensterumschalter herausnimmt. Die Platzierung reist im Fenstertitel mit, der bei diesen
rahmenlosen Fenstern nie sichtbar ist.

Die Erweiterung bringt auch die Erfassungsgeste zurück. Unter GNOME verschiebt `Super`+Ziehen
bereits Fenster, daher lautet die Geste **`Super`+`Alt`+Ziehen**, aktiv, solange
Snipdeck läuft:

| Geste (GNOME) | Ergebnis |
|---|---|
| `Super`+`Alt` + Ziehen | Schwebender Ausschnitt auf dem Bildschirm |
| `Super`+`Alt`+`Ctrl` + Ziehen | Schwebender Ausschnitt **und** Kopie in die Zwischenablage |
| `Super`+`Alt`+`Shift` + Ziehen | Nur Galerie (kein schwebendes Fenster) |
| `Super`+`Alt`+`Ctrl`+`Shift` + Ziehen | Galerie **und** Zwischenablage, kein schwebendes Fenster |

„Nur Zwischenablage“ – `Win`+`Alt` unter Windows – hat hier keine Geste; nutze ihr
Tastenkürzel. `Esc` bricht ein Ziehen ab, und solange du `Super`+`Alt` gedrückt hältst, kannst
du mehrere Ausschnitte nacheinander aufnehmen.

Melde dich nach der Installation ab und wieder an; die GNOME Shell kann Erweiterungen unter
Wayland nicht an Ort und Stelle neu laden, weil sie *selbst* der Compositor ist, der deine
Sitzung trägt. Die Erweiterung ist völlig optional – ohne sie verhält sich Snipdeck wie jeder
andere Wayland-Client – und abgesehen von Snipdecks eigenen Fenstern berührt sie nur
`Super`+`Alt`+Ziehen, und auch das nur, solange Snipdeck läuft.

Unter GNOME Wayland fehlt noch: Einen Ausschnitt, den du ziehst, verschiebt Mutter, und die App
erfährt nie, wo er gelandet ist; daher wird seine Position über Neustarts hinweg nicht
gespeichert.

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
| `upload_service` | `"catbox"` | Bild-Hoster für *Hochladen*: `"catbox"` (kein Schlüssel), `"imgur"` oder `"custom:<name>"`; einstellbar über `⋯` → Hochladen zu |
| `upload_hosts` | `[]` | Aus ShareX-`.sxcu`-Dateien importierte Hoster, mit den Feldnamen von ShareX |
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

## 🪶 Ressourcenverbrauch

Snipdeck ist nativer Code: Rust, mit einer [Slint](https://slint.dev)-Oberfläche.
Es gibt keine eingebettete Browser-Engine (wie bei Electron-Apps) und keine .NET-
oder Java-Laufzeitumgebung, daher kommt es als eine einzige Datei und bleibt auch
im Betrieb schlank.

| | Galerie offen, im Leerlauf | Wartend im Tray |
| --- | --- | --- |
| Arbeitsspeicher (PSS) | 68 MB | 67 MB |
| davon eigener (USS) | 57 MB | 57 MB |
| CPU | 0.6 % | 0.05 % |

| Download | Größe |
| --- | --- |
| Windows, `snipdeck-windows-x64.exe` | 32 MB, die ganze App |
| Linux, `.deb` | 10 MB |

*So wurde gemessen:* der Release-Build 0.3.0 unter Linux x64 (GNOME 51, Wayland),
mit 10 Ausschnitten in der Galerie, nach 25 Sekunden Ruhe; Speicher aus
`/proc/<pid>/smaps_rollup` gelesen, CPU über 40 Sekunden gemittelt. PSS rechnet
die gemeinsam genutzten Bibliotheken (GTK, Mesa) anteilig nach ihren Nutzern an;
USS ist der Speicher, den nur Snipdeck belegt. Gemessen am 15.10.2026; Windows
wurde noch nicht gemessen.

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
| `src/sxcu.rs`, `src/upload_window.rs`, `ui/upload.slint` | ShareX-Custom-Uploader; das Fenster für Upload-Einwilligung / -Fortschritt / -Ergebnis |
| `src/tray.rs`, `src/autostart.rs`, `src/single_instance.rs` | Tray-Symbol, Start bei der Anmeldung, Single-Instance-Schutz |
| `src/snip.rs`, `src/settings.rs`, `src/paths.rs` | Galerie-Persistenz (SQLite + FTS), Einstellungen, Pfade |
| `src/image_files.rs`, `src/atomic_file.rs` | Ausschnittbilder auf der Festplatte: verlustfreies WebP, atomare Schreibvorgänge |
| `src/i18n.rs`, `lang/` | Laufzeit-i18n + eingebettete gettext-`.po`-Kataloge (23 Sprachen) |

</details>

---

## ✨ Entwickelt von ReviseTouch

Snipdeck ist eine kostenlose Open-Source-App von **[ReviseTouch](https://revisetouch.com)** –
wir bauen Entwicklertools und KI-Produkte. Was wir sonst noch bauen, siehst du auf
**[revisetouch.com](https://revisetouch.com)**.

---

## Lizenz

[MIT](LICENSE) © 2026 Hasan Salihoğlu

<div align="center">
<sub>Erstellt mit 🦀 Rust und <a href="https://slint.dev">Slint</a>.</sub>
</div>
