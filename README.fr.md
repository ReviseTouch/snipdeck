<div align="center">

[English](README.md) · [Türkçe](README.tr.md) · [Español](README.es.md) · [简体中文](README.zh-CN.md) · [العربية](README.ar.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · [日本語](README.ja.md) · **Français** · [Português](README.pt.md)

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**Un _deck_ de capture d'écran rapide et piloté au clavier pour Windows.**

Capturez une zone, gardez vos captures flottantes au-dessus de tout, annotez-les,
indexez-les par OCR et recherchez parmi des centaines de captures d'écran depuis une seule galerie dense —
le tout depuis une application Rust native dotée d'une interface [Slint](https://slint.dev) rendue de manière logicielle.

![platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows&logoColor=white)
![soon](https://img.shields.io/badge/macOS%20%2F%20Linux-on%20the%20way-lightgrey)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Slint-orange?logo=rust&logoColor=white)
![languages](https://img.shields.io/badge/languages-23-success)
![license](https://img.shields.io/badge/license-MIT-green)

</div>

---

## 📸 Captures d'écran

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · translate.png
-->

<div align="center">

| L'espace de travail de la galerie | Captures flottantes sur le bureau |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="Galerie" width="420" /> | <img src="docs/screenshots/floating.png" alt="Captures flottantes" width="420" /> |
| **L'éditeur d'annotation** | **OCR + traduction** |
| <img src="docs/screenshots/annotate.png" alt="Éditeur d'annotation" width="420" /> | <img src="docs/screenshots/translate.png" alt="Traduction OCR" width="420" /> |

</div>

---

## ✨ Points forts

- 🖱️ **Capture sur modificateur + glisser.** Maintenez `Win` et faites glisser une zone ; les modificateurs décident de ce qui
  arrive au résultat — fenêtre flottante, presse-papiers, galerie uniquement (matrice ci-dessous).
- 🧊 **Capture par gel d'abord** (`Win`+`Shift`+`Space`). Assombrit et gèle l'écran *avant* votre
  sélection, afin que les info-bulles au survol et autres éléments d'interface sensibles au clic restent capturés.
- 📌 **Captures flottantes.** Les captures restent épinglées au-dessus sous forme de superpositions légères et sans bordure que vous pouvez
  déplacer, redimensionner, épingler, rogner, faire pivoter/retourner et fermer.
- ✏️ **Éditeur d'annotation intégré.** Stylo, surligneur, formes, flèches et texte avec une roue chromatique TSV
  complète, taille et opacité réglables, zoom, et annuler/rétablir (détails).
- 🔍 **OCR intégré.** Chaque capture est indexée par OCR en arrière-plan grâce au moteur du système d'exploitation
  (`Windows.Media.Ocr`) — aucun téléchargement de modèle, aucun réseau, entièrement local.
- 🌐 **OCR + traduction.** Lisez le texte d'une capture dans une autre langue directement sur place (plus de 30 cibles, avec
  des favoris que vous pouvez marquer d'une étoile).
- 🗂️ **Galerie puissante.** Recherche en texte intégral sur le texte OCR, les métadonnées de fenêtre, le moniteur et le chemin ;
  filtrage par statut / contenu / taille / date ; une grille virtualisée qui reste fluide même avec des milliers de captures.
- 📤 **Partage et envoi.** Volet de partage Windows, un nouvel e-mail (Simple MAPI), l'éditeur d'images du système,
  ou un hébergeur d'images — les envois sont **facultatifs avec une boîte de dialogue de consentement explicite**.
- 🌍 **23 langues**, détectées automatiquement depuis votre système d'exploitation et commutables depuis la barre d'outils.
- 🌗 **Thèmes clair / sombre**, un accueil dans la zone de notification, et un lancement au démarrage **sans registre**.

---

## ⌨️ Raccourcis de capture

Maintenez `Win`, puis faites glisser une zone avec le bouton **gauche** de la souris. Ajoutez des modificateurs pour changer ce qui
arrive à la capture :

| Raccourci | Résultat |
| --- | --- |
| `Win` + glisser | Capture flottante à l'écran |
| `Win`+`Ctrl` + glisser | Capture flottante **et** copie dans le presse-papiers |
| `Win`+`Alt` + glisser | Presse-papiers uniquement (non ajoutée à la galerie) |
| `Win`+`Shift` + glisser | Galerie uniquement (pas de fenêtre flottante) |
| `Win`+`Ctrl`+`Shift` + glisser | Galerie **et** presse-papiers, pas de fenêtre flottante |
| `Win`+`Shift`+`Space`, puis glisser | **Gel d'abord** — gèle l'écran, puis sélectionnez sur l'image figée |

Vous pouvez aussi faire un clic droit sur l'icône de la zone de notification → **Nouvelle capture** pour armer une capture par glisser unique sans
maintenir `Win`.

### Raccourcis clavier globaux

Outre `Win` + glisser, Snipdeck enregistre cinq raccourcis clavier globaux configurables :

| Raccourci | Action |
| --- | --- |
| `Win`+`Alt`+`H` | Afficher / masquer toutes les captures à l'écran |
| `Ctrl`+`Shift`+`A` | Nouvelle capture |
| `Ctrl`+`Shift`+`C` | Nouvelle capture **et** copie dans le presse-papiers |
| `Ctrl`+`Shift`+`X` | Presse-papiers uniquement |
| `Ctrl`+`Shift`+`G` | Ouvrir la galerie |

Modifiez-les avec les clés `hotkey_*` dans `settings.json`.

---

## ✏️ Éditeur d'annotation

Faites un clic droit sur n'importe quelle capture → **Annoter** pour ouvrir un éditeur ciblé, façon Telegram, qui se place au-dessus
de vos captures.

- **Outils** — stylo à main levée, surligneur, rectangle, ellipse, ligne, flèche et texte.
- **Roue chromatique TSV complète** avec un curseur de luminosité — choisissez *n'importe quelle* couleur, pas une palette figée.
- **Curseurs de taille et d'opacité** avec un aperçu de taille clignotant sur l'image, mémorisé par outil.
- **Zoom avant / arrière / ajuster** pour un travail précis ; le canevas se déplace lorsqu'on est zoomé.
- **Annuler / rétablir / effacer**, ainsi que `Esc`, `Ctrl`+`Z`, `Ctrl`+`Y`.
- À l'**enregistrement**, les tracés sont aplatis sur l'image en pleine résolution (nets, à taille native).

L'aperçu en direct et le résultat enregistré proviennent du même code, donc **ce que vous voyez est ce que vous obtenez**.

---

## 🖱️ Actions du clic droit

Faites un clic droit sur une capture flottante **ou** une vignette de la galerie :

- **Copier** l'image dans le presse-papiers
- **OCR + Presse-papiers** — lance l'OCR et copie le texte reconnu
- **OCR + Traduction** — ouvre la fenêtre de traduction
- **Annoter** — ouvre l'éditeur ci-dessus
- **Partager** — volet de partage Windows, **Courrier** (pièce jointe MAPI), ouvrir dans l'**éditeur** (Paint),
  ou **Envoyer** vers un hébergeur d'images
- **Transformer** — pivoter (90° / 180°), retourner horizontalement / verticalement
- **Rogner** (faites glisser les poignées vers l'intérieur) et **Redimensionner**
- **Masquer de l'écran** / **Supprimer**

---

## 📦 Installation

### Téléchargement

Récupérez le dernier `snipdeck.exe` depuis la page [**Releases**](https://github.com/hasantr/snipdeck/releases).
C'est un unique exécutable portable — pas d'installateur.

### Compilation depuis les sources

Prérequis : [Rust](https://rustup.rs/) (stable) et Windows 10 ou 11.

```powershell
git clone https://github.com/hasantr/snipdeck.git
cd snipdeck
cargo build --release
```

Le binaire se retrouve dans `target\release\snipdeck.exe`.

---

## 🌍 Langues

Snipdeck est entièrement localisé. Au premier lancement, il choisit votre langue selon les paramètres régionaux du système d'exploitation ;
modifiez-la à tout moment depuis le **menu globe** de la barre d'outils.

> Anglais · Espagnol · Chinois simplifié · Hindi · Arabe · Portugais · Russe · Japonais ·
> Allemand · Français · Coréen · Italien · Turc · Indonésien · Vietnamien · Polonais · Ukrainien ·
> Néerlandais · Persan · Thaï · Bengali · Ourdou · Roumain

L'application entière — à la fois l'interface Slint (`@tr`) et les messages côté Rust (`i18n::t`) — lit depuis
**un seul fichier gettext `.po` par langue** sous `lang/<code>/LC_MESSAGES/snipdeck.po`, intégré dans le
binaire au moment de la compilation (aucune dépendance gettext à l'exécution).

<details>
<summary><b>Ajouter ou mettre à jour une traduction</b></summary>

1. Les nouvelles chaînes sont enveloppées dans `@tr("…")` (dans `.slint`) ou `i18n::t("…")` (en Rust).
2. `lang/snipdeck.pot` est le modèle qui répertorie chaque chaîne anglaise.
3. Copiez un nouveau msgid dans chaque `lang/<code>/LC_MESSAGES/snipdeck.po` et remplissez son `msgstr` —
   gardez les espaces réservés (`{}`, `{e}`, `{url}`, `{path}`) et les séquences d'échappement `\r\n` intacts.
4. Pour ajouter une nouvelle langue, créez son `.po` et ajoutez le code à `LANGUAGES` et `po_source()`
   dans `src/i18n.rs`.

</details>

---

## 🔒 Confidentialité et sécurité

- **La capture et l'OCR sont entièrement locaux.** L'OCR utilise le moteur intégré `Windows.Media.Ocr` ; rien
  n'est envoyé où que ce soit pour la capture ou l'indexation.
- **Le lancement au démarrage est sans registre.** Snipdeck dépose un raccourci normal dans votre dossier `shell:Startup`
  au lieu d'écrire une valeur `HKCU\…\Run` — le mécanisme ordinaire et visible par l'utilisateur qui
  évite la clé de registre de démarrage automatique que les outils antivirus / EDR signalent le plus souvent.
- **Les envois sont facultatifs.** Rien ne quitte votre machine sans une boîte de dialogue de consentement explicite qui
  précise la destination. N'envoyez pas de contenu sensible vers un hébergeur public par lien.
- **OCR + Traduction utilise le réseau.** Seul le *texte* issu de l'OCR est envoyé au point de terminaison public
  `translate_a` de Google pour être traduit ; il n'est utilisé nulle part ailleurs.

---

## 🗺️ Feuille de route

- [x] Windows 10 / 11
- [ ] **macOS** — en chemin
- [ ] **Linux** — en chemin
- [ ] Fenêtre de paramètres dans l'application
- [ ] Plus d'outils d'annotation (flou / pixelisation, badges d'étapes numérotés, déplacer/modifier des formes)

> Snipdeck est aujourd'hui réservé à Windows car la capture, l'OCR, la zone de notification et le partage utilisent tous des
> API spécifiques à Windows. L'interface ([Slint](https://slint.dev)) et la majeure partie du cœur sont déjà
> multiplateformes, donc **les versions macOS et Linux sont en chemin.**

---

## ⚙️ Configuration

Les paramètres sont stockés au format JSON et créés au premier lancement. Il n'y a pas encore de fenêtre de paramètres — modifiez le
fichier directement, puis redémarrez Snipdeck.

| Chemin | Contenu |
| --- | --- |
| `%APPDATA%\Snipdeck\settings.json` | Paramètres utilisateur (ci-dessous) |
| `%LOCALAPPDATA%\Snipdeck\` | `snips.db`, `state.json`, et le `cache\` d'images + vignettes |
| `Pictures\Snipdeck\` | Dossier par défaut pour *Enregistrer* |

<details>
<summary><b>Quelques clés de <code>settings.json</code></b></summary>

| Clé | Défaut | Signification |
| --- | --- | --- |
| `language` | `"auto"` | Code de langue de l'interface, ou `"auto"` pour suivre le système d'exploitation |
| `dark_theme` | `true` | Habillage clair ou sombre |
| `upload_service` | `"catbox"` | Hébergeur d'images pour *Envoyer* : `"catbox"` (sans clé) ou `"imgur"` |
| `imgur_client_id` | `""` | Client-ID Imgur, requis uniquement lorsque `upload_service` vaut `"imgur"` |
| `translate_target` | `"en"` | Code de langue cible par défaut pour la traduction OCR |
| `translate_favs` | `["en","es","fr","de","it","pt","ru","ar"]` | Langues à sélection rapide dans la fenêtre de traduction |
| `default_format` | `"png"` | Extension par défaut pour *Enregistrer* |

**Services d'envoi** — *catbox.moe* (par défaut) ne nécessite ni compte ni clé, mais les envois anonymes
**ne peuvent pas être supprimés par vous** par la suite. *Imgur* nécessite un Client-ID gratuit (inscription unique sur
imgur.com → Settings → Applications). Dans les deux cas, l'image n'est pas répertoriée mais quiconque dispose du
lien peut l'ouvrir, et elle n'est pas chiffrée.

</details>

---

## 🧩 Architecture

Une boucle d'événements Slint mono-thread pilote l'interface ; les hooks du système d'exploitation et les tâches bloquantes s'exécutent sur leurs propres
threads et renvoient les résultats via des canaux, vidés une fois par tick.

<details>
<summary><b>Carte des modules</b></summary>

| Module | Responsabilité |
| --- | --- |
| `src/app/` | État de l'application, la boucle par tick, les actions d'interface, le modèle de galerie |
| `src/capture.rs`, `src/dxgi_capture.rs` | Capture du moniteur (DXGI Desktop Duplication, repli GDI) |
| `src/mouse_trigger.rs`, `src/native_mouse_trigger.rs` | Hooks souris/clavier bas niveau qui arment et pilotent les captures |
| `src/slint_overlay.rs`, `ui/overlay.slint` | Superposition de sélection plein écran (en direct ou figée) |
| `src/floating.rs`, `ui/floating.slint` | Fenêtres de captures flottantes (déplacer, redimensionner, rogner, bordure) |
| `src/annotate.rs`, `ui/annotate.slint` | Éditeur d'annotation (aplatissement tiny-skia, roue TSV, zoom) |
| `src/ocr.rs` | Encapsulation de `Windows.Media.Ocr` (sur un thread de travail) |
| `src/translate.rs`, `src/translate_popup.rs` | Fenêtre de traduction OCR |
| `src/share.rs`, `src/upload.rs` | Volet de partage / courrier MAPI / éditeur / envoi vers hébergeur d'images |
| `src/tray.rs`, `src/autostart.rs`, `src/single_instance.rs` | Icône de zone de notification, lancement au démarrage, garde d'instance unique |
| `src/snip.rs`, `src/settings.rs`, `src/paths.rs` | Persistance de la galerie (SQLite + FTS), paramètres, chemins |
| `src/i18n.rs`, `lang/` | i18n à l'exécution + catalogues gettext `.po` intégrés (23 langues) |

</details>

---

## Licence

[MIT](LICENSE) © 2026 Hasan Salihoğlu

<div align="center">
<sub>Conçu avec 🦀 Rust et <a href="https://slint.dev">Slint</a>.</sub>
</div>
