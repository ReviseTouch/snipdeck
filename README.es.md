<div align="center">

[English](README.md) · [Türkçe](README.tr.md) · **Español** · [简体中文](README.zh-CN.md) · [العربية](README.ar.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · [日本語](README.ja.md) · [Français](README.fr.md) · [Português](README.pt.md)

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**Un _deck_ de recortes de pantalla rápido y controlado por teclado para Windows.**

Captura una región, mantén los recortes flotando por encima de todo, anótalos,
indéxalos con OCR y busca entre cientos de capturas de pantalla desde una única galería densa,
todo desde una aplicación nativa en Rust con una interfaz [Slint](https://slint.dev) renderizada por software.

![platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows&logoColor=white)
![soon](https://img.shields.io/badge/macOS%20%2F%20Linux-on%20the%20way-lightgrey)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Slint-orange?logo=rust&logoColor=white)
![languages](https://img.shields.io/badge/languages-23-success)
![license](https://img.shields.io/badge/license-MIT-green)

</div>

---

## 📸 Capturas de pantalla

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · translate.png
-->

<div align="center">

| El espacio de trabajo de la galería | Recortes flotantes en el escritorio |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="Galería" width="420" /> | <img src="docs/screenshots/floating.png" alt="Recortes flotantes" width="420" /> |
| **El editor de anotaciones** | **OCR + traducción** |
| <img src="docs/screenshots/annotate.png" alt="Editor de anotaciones" width="420" /> | <img src="docs/screenshots/translate.png" alt="Traducción con OCR" width="420" /> |

</div>

---

## ✨ Aspectos destacados

- 🖱️ **Captura con un modificador + arrastre.** Mantén pulsada `Win` y arrastra una región; los modificadores deciden qué
  le ocurre al resultado: ventana flotante, portapapeles, solo galería (matriz más abajo).
- 🧊 **Captura con congelación previa** (`Win`+`Shift`+`Space`). Atenúa y congela la pantalla *antes* de que
  selecciones, de modo que las descripciones emergentes al pasar el cursor y otra interfaz sensible al clic se mantienen capturadas.
- 📌 **Recortes flotantes.** Las capturas quedan fijadas por encima de todo como superposiciones ligeras y sin bordes que puedes
  arrastrar, redimensionar, fijar, recortar, rotar/voltear y descartar.
- ✏️ **Editor de anotaciones integrado.** Lápiz, marcador, formas, flechas y texto con una rueda
  de colores HSV completa, tamaño y opacidad ajustables, zoom y deshacer/rehacer (detalles).
- 🔍 **OCR integrado.** Cada recorte se indexa con OCR en segundo plano mediante el motor del sistema operativo
  (`Windows.Media.Ocr`): sin descarga de modelos, sin red, totalmente local.
- 🌐 **OCR + traducción.** Lee el texto de un recorte en otro idioma de forma integrada (más de 30 destinos, con
  favoritos que puedes marcar con una estrella).
- 🗂️ **Galería potente.** Búsqueda de texto completo sobre el texto OCR, los metadatos de la ventana, el monitor y la ruta;
  filtra por estado / contenido / tamaño / fecha; una cuadrícula virtualizada que se mantiene fluida con miles de recortes.
- 📤 **Compartir y subir.** Hoja de compartir de Windows, un nuevo correo electrónico (Simple MAPI), el editor de imágenes del sistema,
  o un alojamiento de imágenes: las subidas son **opcionales y con un diálogo de consentimiento explícito**.
- 🌍 **23 idiomas**, detectados automáticamente desde tu sistema operativo y conmutables desde la barra de herramientas.
- 🌗 **Temas claro / oscuro**, una base en la bandeja del sistema y un arranque automático al iniciar sesión **sin registro**.

---

## ⌨️ Atajos de captura

Mantén pulsada `Win` y luego arrastra una región con el botón **izquierdo** del ratón. Añade modificadores para cambiar qué
le ocurre al recorte:

| Atajo | Resultado |
| --- | --- |
| `Win` + arrastre | Recorte flotante en pantalla |
| `Win`+`Ctrl` + arrastre | Recorte flotante **y** copia al portapapeles |
| `Win`+`Alt` + arrastre | Solo portapapeles (no se añade a la galería) |
| `Win`+`Shift` + arrastre | Solo galería (sin ventana flotante) |
| `Win`+`Ctrl`+`Shift` + arrastre | Galería **y** portapapeles, sin ventana flotante |
| `Win`+`Shift`+`Space`, luego arrastre | **Congelación previa**: congela la pantalla y luego selecciona sobre la imagen fija |

También puedes hacer clic derecho en el icono de la bandeja → **Nuevo recorte** para armar una captura de un solo arrastre sin
mantener pulsada `Win`.

### Atajos globales

Además de `Win` + arrastre, Snipdeck registra cinco atajos globales configurables:

| Atajo | Acción |
| --- | --- |
| `Win`+`Alt`+`H` | Mostrar / ocultar todos los recortes en pantalla |
| `Ctrl`+`Shift`+`A` | Nuevo recorte |
| `Ctrl`+`Shift`+`C` | Nuevo recorte **y** copia al portapapeles |
| `Ctrl`+`Shift`+`X` | Solo portapapeles |
| `Ctrl`+`Shift`+`G` | Abrir la galería |

Cámbialos con las claves `hotkey_*` en `settings.json`.

---

## ✏️ Editor de anotaciones

Haz clic derecho en cualquier recorte → **Anotar** para abrir un editor enfocado, estilo Telegram, que se sitúa por encima
de tus recortes.

- **Herramientas**: lápiz a mano alzada, marcador, rectángulo, elipse, línea, flecha y texto.
- **Rueda de colores HSV completa** con un control deslizante de brillo: elige *cualquier* color, no una paleta fija.
- **Controles deslizantes de tamaño y opacidad** con una vista previa parpadeante del tamaño sobre la imagen, recordada por herramienta.
- **Acercar / alejar / ajustar** para un trabajo preciso; el lienzo se desplaza cuando se amplía.
- **Deshacer / rehacer / borrar**, además de `Esc`, `Ctrl`+`Z`, `Ctrl`+`Y`.
- Al **Guardar**, los trazos se aplanan sobre la imagen a resolución completa (nítidos, a tamaño nativo).

La vista previa en vivo y el resultado guardado provienen del mismo código, así que **lo que ves es lo que obtienes**.

---

## 🖱️ Acciones con clic derecho

Haz clic derecho en un recorte flotante **o** en una miniatura de la galería:

- **Copiar** la imagen al portapapeles
- **OCR + portapapeles**: ejecuta el OCR y copia el texto reconocido
- **OCR + traducir**: abre la ventana emergente de traducción
- **Anotar**: abre el editor anterior
- **Compartir**: hoja de compartir de Windows, **Correo** (adjunto MAPI), abrir en el **editor** (Paint),
  o **Subir** a un alojamiento de imágenes
- **Transformar**: rotar (90° / 180°), voltear horizontal / vertical
- **Recortar** (arrastra los tiradores hacia dentro) y **Redimensionar**
- **Ocultar de la pantalla** / **Eliminar**

---

## 📦 Instalación

### Descarga

Consigue el último `snipdeck.exe` desde la página de [**Releases**](https://github.com/hasantr/snipdeck/releases).
Es un único ejecutable portátil, sin instalador.

### Compilar desde el código fuente

Requisitos: [Rust](https://rustup.rs/) (estable) y Windows 10 u 11.

```powershell
git clone https://github.com/hasantr/snipdeck.git
cd snipdeck
cargo build --release
```

El binario se genera en `target\release\snipdeck.exe`.

---

## 🌍 Idiomas

Snipdeck está totalmente localizado. En el primer inicio elige tu idioma a partir de la configuración regional del sistema operativo;
puedes cambiarlo en cualquier momento desde el **menú del globo** de la barra de herramientas.

> Inglés · Español · Chino simplificado · Hindi · Árabe · Portugués · Ruso · Japonés ·
> Alemán · Francés · Coreano · Italiano · Turco · Indonesio · Vietnamita · Polaco · Ucraniano ·
> Neerlandés · Persa · Tailandés · Bengalí · Urdu · Rumano

Toda la aplicación —tanto la interfaz Slint (`@tr`) como los mensajes del lado de Rust (`i18n::t`)— lee de
**un único `.po` de gettext por idioma** en `lang/<code>/LC_MESSAGES/snipdeck.po`, integrado en el
binario en tiempo de compilación (sin dependencia de gettext en tiempo de ejecución).

<details>
<summary><b>Añadir o actualizar una traducción</b></summary>

1. Las cadenas nuevas se envuelven en `@tr("…")` (en `.slint`) o `i18n::t("…")` (en Rust).
2. `lang/snipdeck.pot` es la plantilla que enumera todas las cadenas en inglés.
3. Copia un nuevo msgid en cada `lang/<code>/LC_MESSAGES/snipdeck.po` y rellena su `msgstr`:
   mantén intactos los marcadores de posición (`{}`, `{e}`, `{url}`, `{path}`) y los escapes `\r\n`.
4. Para añadir un nuevo idioma, crea su `.po` y añade el código a `LANGUAGES` y `po_source()`
   en `src/i18n.rs`.

</details>

---

## 🔒 Privacidad y seguridad

- **La captura y el OCR son totalmente locales.** El OCR usa el motor `Windows.Media.Ocr` integrado; nada
  se envía a ningún sitio para la captura o la indexación.
- **El arranque automático al iniciar sesión es sin registro.** Snipdeck deja un acceso directo normal en tu carpeta `shell:Startup`
  en lugar de escribir un valor `HKCU\…\Run`: el mecanismo ordinario y visible para el usuario que
  evita la clave de registro de autoinicio que las herramientas de antivirus / EDR marcan con más frecuencia.
- **Las subidas son opcionales.** Nada sale de tu equipo sin un diálogo de consentimiento explícito que
  detalla el destino. No subas contenido sensible a un alojamiento público mediante enlace.
- **OCR + traducir usa la red.** Solo el *texto* del OCR se envía al endpoint público
  `translate_a` de Google para ser traducido; no se utiliza en ningún otro lugar.

---

## 🗺️ Hoja de ruta

- [x] Windows 10 / 11
- [ ] **macOS** — en camino
- [ ] **Linux** — en camino
- [ ] Ventana de ajustes dentro de la aplicación
- [ ] Más herramientas de anotación (desenfoque / pixelado, insignias numeradas de pasos, mover/editar formas)

> Hoy Snipdeck es solo para Windows porque la captura, el OCR, la bandeja y la función de compartir usan todos
> API específicas de Windows. La interfaz ([Slint](https://slint.dev)) y la mayor parte del núcleo ya son
> multiplataforma, así que **las versiones para macOS y Linux están en camino.**

---

## ⚙️ Configuración

Los ajustes se almacenan como JSON y se crean en la primera ejecución. Todavía no hay una ventana de ajustes: edita el
archivo directamente y luego reinicia Snipdeck.

| Ruta | Contenido |
| --- | --- |
| `%APPDATA%\Snipdeck\settings.json` | Ajustes del usuario (abajo) |
| `%LOCALAPPDATA%\Snipdeck\` | `snips.db`, `state.json` y la `cache\` de imágenes + miniaturas |
| `Pictures\Snipdeck\` | Carpeta predeterminada para *Guardar* |

<details>
<summary><b>Claves seleccionadas de <code>settings.json</code></b></summary>

| Clave | Predeterminado | Significado |
| --- | --- | --- |
| `language` | `"auto"` | Código de idioma de la interfaz, o `"auto"` para seguir al sistema operativo |
| `dark_theme` | `true` | Tema claro u oscuro |
| `upload_service` | `"catbox"` | Alojamiento de imágenes para *Subir*: `"catbox"` (sin clave) o `"imgur"` |
| `imgur_client_id` | `""` | Client-ID de Imgur, requerido solo cuando `upload_service` es `"imgur"` |
| `translate_target` | `"en"` | Código de idioma de destino predeterminado para la traducción con OCR |
| `translate_favs` | `["en","es","fr","de","it","pt","ru","ar"]` | Idiomas de selección rápida en la ventana emergente de traducción |
| `default_format` | `"png"` | Extensión predeterminada para *Guardar* |

**Servicios de subida**: *catbox.moe* (predeterminado) no necesita cuenta ni clave, pero las subidas anónimas
**no puedes eliminarlas tú** posteriormente. *Imgur* necesita un Client-ID gratuito (regístrate una vez en
imgur.com → Settings → Applications). En ambos casos la imagen no aparece en listados, pero cualquiera con el
enlace puede abrirla, y no está cifrada.

</details>

---

## 🧩 Arquitectura

Un bucle de eventos de Slint de un solo hilo impulsa la interfaz; los hooks del sistema operativo y el trabajo bloqueante se ejecutan en sus propios
hilos y devuelven los resultados a través de canales, vaciados una vez por tick.

<details>
<summary><b>Mapa de módulos</b></summary>

| Módulo | Responsabilidad |
| --- | --- |
| `src/app/` | Estado de la aplicación, el bucle por tick, las acciones de la interfaz, el modelo de la galería |
| `src/capture.rs`, `src/dxgi_capture.rs` | Captura de monitor (DXGI Desktop Duplication, GDI como alternativa) |
| `src/mouse_trigger.rs`, `src/native_mouse_trigger.rs` | Hooks de ratón/teclado de bajo nivel que arman e impulsan los recortes |
| `src/slint_overlay.rs`, `ui/overlay.slint` | Superposición de selección a pantalla completa (en vivo o congelada) |
| `src/floating.rs`, `ui/floating.slint` | Ventanas de recortes flotantes (arrastrar, redimensionar, recortar, borde) |
| `src/annotate.rs`, `ui/annotate.slint` | Editor de anotaciones (aplanado con tiny-skia, rueda HSV, zoom) |
| `src/ocr.rs` | Envoltorio de `Windows.Media.Ocr` (en un hilo de trabajo) |
| `src/translate.rs`, `src/translate_popup.rs` | Ventana emergente de traducción con OCR |
| `src/share.rs`, `src/upload.rs` | Hoja de compartir / correo MAPI / editor / subida a alojamiento de imágenes |
| `src/tray.rs`, `src/autostart.rs`, `src/single_instance.rs` | Icono de la bandeja, arranque al iniciar sesión, protección de instancia única |
| `src/snip.rs`, `src/settings.rs`, `src/paths.rs` | Persistencia de la galería (SQLite + FTS), ajustes, rutas |
| `src/i18n.rs`, `lang/` | i18n en tiempo de ejecución + catálogos `.po` de gettext integrados (23 idiomas) |

</details>

---

## Licencia

[MIT](LICENSE) © 2026 Hasan Salihoğlu

<div align="center">
<sub>Construido con 🦀 Rust y <a href="https://slint.dev">Slint</a>.</sub>
</div>
