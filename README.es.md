<div align="center">

[English](README.md) · [Türkçe](README.tr.md) · **Español** · [简体中文](README.zh-CN.md) · [العربية](README.ar.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · [日本語](README.ja.md) · [Français](README.fr.md) · [Português](README.pt.md)

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**Un _deck_ de recortes de pantalla rápido y controlado por teclado para Windows.**

<sub>Una aplicación gratuita y de código abierto de <a href="https://revisetouch.com"><b>ReviseTouch</b></a>.</sub>

Captura una región, mantén los recortes flotando por encima de todo, anótalos,
indéxalos con OCR y busca entre cientos de capturas de pantalla desde una única galería densa,
todo desde una aplicación nativa en Rust con una interfaz [Slint](https://slint.dev) renderizada por software.

![platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows&logoColor=white)
![linux](https://img.shields.io/badge/Linux-GNOME%20full%20%2F%20others%20partial-yellow) ![soon](https://img.shields.io/badge/macOS-on%20the%20way-lightgrey)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Slint-orange?logo=rust&logoColor=white)
![languages](https://img.shields.io/badge/languages-23-success)
![license](https://img.shields.io/badge/license-MIT-green)

</div>

---

## 📸 Capturas de pantalla

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · collage.png
-->

<div align="center">

| El espacio de trabajo de la galería | Recortes flotantes en el escritorio |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="Galería" width="420" /> | <img src="docs/screenshots/floating.png" alt="Recortes flotantes" width="420" /> |
| **El editor de anotaciones** | **El editor de collages** |
| <img src="docs/screenshots/annotate.png" alt="Editor de anotaciones" width="420" /> | <img src="docs/screenshots/collage.png" alt="Editor de collages" width="420" /> |

</div>

---

## ✨ Aspectos destacados

- 🖱️ **Captura con un modificador + arrastre.** Mantén pulsada `Win` y arrastra una región; los modificadores deciden qué
  le ocurre al resultado: ventana flotante, portapapeles, solo galería ([matriz más abajo](#️-atajos-de-captura)).
- 🧊 **Captura con congelación previa** (`Win`+`Shift`+`Space`). Atenúa y congela la pantalla *antes* de que
  selecciones, de modo que las descripciones emergentes al pasar el cursor y otra interfaz sensible al clic se mantienen capturadas.
- 📌 **Recortes flotantes.** Las capturas quedan fijadas por encima de todo como superposiciones ligeras y sin bordes que puedes
  arrastrar, redimensionar, fijar, recortar, rotar/voltear y descartar.
- ✏️ **Editor de anotaciones integrado.** Lápiz, marcador, formas, flechas y texto con una rueda
  de colores HSV completa, tamaño y opacidad ajustables, zoom y deshacer/rehacer ([detalles](#️-editor-de-anotaciones)).
- 🧩 **Collage.** Dispón varios recortes en una sola imagen (cuadrícula, filas justificadas o mosaico tipo masonry,
  con espacio, esquinas redondeadas y fondo), que se guarda en la galería como un recorte nuevo.
- 🔍 **OCR integrado.** Cada recorte se indexa con OCR en segundo plano mediante el motor del sistema operativo
  (`Windows.Media.Ocr`): sin descarga de modelos, sin red, totalmente local.
- 🌐 **OCR + traducción.** Lee el texto de un recorte en otro idioma de forma integrada (29 destinos, con
  favoritos que puedes marcar con una estrella).
- 🗂️ **Galería potente.** Búsqueda de texto completo sobre el texto OCR, los metadatos de la ventana, el monitor y la ruta;
  filtra por estado / contenido / tamaño / fecha; una cuadrícula virtualizada que se mantiene fluida con miles de recortes
  ([recorrido](#️-la-galería)).
- 📤 **Compartir y subir.** Hoja de compartir de Windows, un nuevo correo electrónico (Simple MAPI), el editor de imágenes del sistema,
  o un alojamiento de imágenes: catbox.moe, Imgur o **tu propio servidor** a partir de un archivo `.sxcu` de ShareX
  ([detalles](#-tu-propio-alojamiento-de-imágenes)). Cada subida **pregunta antes**.
- 🌍 **23 idiomas**, detectados automáticamente desde tu sistema operativo y conmutables desde el menú `⋯` de la barra de herramientas.
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
| `Ctrl`+`Alt`+`Shift`+`A` | Nuevo recorte |
| `Ctrl`+`Alt`+`Shift`+`C` | Nuevo recorte **y** copia al portapapeles |
| `Ctrl`+`Alt`+`Shift`+`X` | Solo portapapeles |
| `Ctrl`+`Alt`+`Shift`+`G` | Abrir la galería |

Cámbialos con las claves `hotkey_*` en `settings.json`. En Wayland, los atajos
globales los gestiona el escritorio: solo se proponen en el primer inicio y,
después, se cambian en los ajustes de teclado de tu escritorio.

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
- **OCR + Portapapeles**: ejecuta el OCR y copia el texto reconocido
- **OCR + Traducir**: abre la ventana emergente de traducción
- **Anotar**: abre el editor anterior
- **Compartir**: hoja de compartir de Windows, **Correo** (adjunto MAPI), abrir en el **editor** (Paint),
  o **Subir** a un alojamiento de imágenes ([a cuál](#-tu-propio-alojamiento-de-imágenes))
- **Transformar**: rotar (90° / 180°), voltear horizontal / vertical
- **Recortar** (arrastra los tiradores hacia dentro) y **Redimensionar**
- **Ocultar de la pantalla** / **Eliminar**

---

## 🗂️ La galería

Una sola fila en la barra de herramientas; todo lo demás está a un clic.

- **Nuevo** hace un recorte. Su flecha contiene **Recorte + Copiar** y **Solo copiar**, cada uno con su atajo.
- **Collage** y **Elegir color** están a su lado, y el cuadro de búsqueda ocupa el resto de la fila:
  `Ctrl`+`F` para enfocarlo, `Esc` para vaciarlo.
- **Filtrar** reúne estado, contenido, tamaño y fecha en un solo panel. Los filtros activos aparecen como etiquetas
  bajo la barra de herramientas, y se pueden quitar uno a uno o todos a la vez con **Quitar filtros**.
- **`⋯`** guarda lo que cambia rara vez: recortes flotantes en la barra de tareas, tema oscuro, iniciar al
  arrancar, idioma, vaciar la galería y Acerca de Snipdeck.
- La barra de estado cuenta lo que se muestra y contiene **Ordenar** y el tamaño de tarjeta **S / M / L**.

Pasa el cursor sobre una tarjeta para ver sus acciones rápidas; haz clic derecho en ella para [todo lo demás](#️-acciones-con-clic-derecho).

---

## 📤 Tu propio alojamiento de imágenes

**`⋯` → Subir a** elige adónde envía **Subir** un recorte: catbox.moe (el predeterminado, sin cuenta),
Imgur o un alojamiento propio. Para añadir uno, elige **Añadir desde un archivo .sxcu…** y selecciona un archivo
de subida personalizada de ShareX: el formato que ya ofrecen Zipline, XBackBone, Chevereto y la mayoría de los
servidores de imágenes autoalojados, así que no hay nada que escribir a mano.

Snipdeck envía la imagen como datos de formulario multipart o como cuerpo binario, con los encabezados, argumentos
y parámetros de consulta del archivo, y lee el enlace con `{json:…}`, `{regex:…}`, `{header:…}` o `{response}`,
también en la forma `$json:…$` de ShareX 12. Un archivo que necesita más (una petición de datos, un valor aleatorio,
una respuesta XML, un destino que no es un alojamiento de imágenes) se rechaza al importarlo, indicando el motivo,
en lugar de fallar en la primera subida.

Antes de enviar nada, la propia ventana de Snipdeck indica el alojamiento y su dirección y explica lo que significa
la subida; al terminar, muestra el enlace y ya lo ha copiado.

---

## 📦 Instalación

### Descarga

**Windows**: consigue `snipdeck-windows-x64.exe` desde la página de
[**Releases**](https://github.com/ReviseTouch/snipdeck/releases). Es un único
ejecutable portátil, sin instalador.

**Linux (x64)**: desde la misma página, el paquete para tu distribución:

```bash
sudo apt install ./snipdeck_*_amd64.deb      # Debian, Ubuntu, Mint
sudo dnf install ./snipdeck-*.x86_64.rpm     # Fedora, RHEL, Rocky
sudo zypper install ./snipdeck-*.x86_64.rpm  # openSUSE
```

o bien, en cualquier distribución y sin root, el tarball:

```bash
tar xzf snipdeck-linux-x64.tar.gz
cd snipdeck-linux-x64
./install.sh          # into ~/.local; --uninstall reverses it
```

Los paquetes funcionan en cualquier distribución con glibc 2.30 o posterior (Debian 11,
Ubuntu 20.04, RHEL 9 y todo lo más reciente) y se han instalado y abierto en Debian 12 y
13, Ubuntu 22.04 y 24.04, Fedora, Rocky 9, openSUSE Leap 15.6 y Tumbleweed
(`packaging/linux/test-install.sh`). En GNOME y KDE Plasma también instalan el asistente
de escritorio de Snipdeck (ver más abajo), que Snipdeck activa en su primer arranque;
GNOME carga una extensión recién instalada en el siguiente inicio de sesión.

### Compilar desde el código fuente

Requisitos: [Rust](https://rustup.rs/) (estable).

```bash
git clone https://github.com/ReviseTouch/snipdeck.git
cd snipdeck
cargo build --release
```

**Windows** (10 u 11): el binario se genera en `target\release\snipdeck.exe`.

**Linux**: la compilación necesita las cabeceras de GTK, X11/Wayland, la bandeja del sistema, D-Bus y GL.
En Debian/Ubuntu:

```bash
sudo apt install libgtk-3-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxcb-randr0-dev libxrandr-dev libxkbcommon-dev \
  libwayland-dev libayatana-appindicator3-dev libfontconfig-dev libdbus-1-dev \
  libgl1-mesa-dev libegl1-mesa-dev
```

PipeWire no necesita nada en tiempo de compilación: se carga en tiempo de ejecución
para la captura en Wayland. Para instalar tu compilación igual que se instala una
versión publicada, colócala junto al instalador:

```bash
packaging/linux/install-from-source.sh   # builds, stages afresh, installs
```

---

## 🌍 Idiomas

Snipdeck está totalmente localizado. En el primer inicio elige tu idioma a partir de la configuración regional del sistema operativo;
puedes cambiarlo en cualquier momento desde **`⋯` → Idioma** en la barra de herramientas.

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
5. Ejecuta `tools/i18n-check.py`: comprueba que cada texto esté traducido con sus marcadores intactos; la CI también lo ejecuta.

</details>

---

## 🔒 Privacidad y seguridad

- **La captura y el OCR son locales.** El OCR usa `Windows.Media.Ocr` en Windows y
  Tesseract en Linux (alimentado mediante una tubería, sin archivo temporal); nada se
  envía a ningún sitio para la captura o la indexación. En Wayland, la pantalla se lee
  a través del portal ScreenCast: lo autorizas una vez, y el indicador de pantalla
  compartida solo aparece en el instante en que se hace un recorte.
- **OCR + Traducir pregunta primero.** La primera vez explica que el *texto* reconocido
  por OCR —nunca la imagen— se envía a Google Traductor, y espera tu confirmación.
  El texto se envía en el cuerpo de la solicitud, no en la URL.
- **Las subidas preguntan antes, siempre.** Nada sale de tu equipo hasta que pulsas
  **Subir** en una ventana que indica el alojamiento y su dirección y explica lo que
  eso significa: cualquiera con el enlace puede abrir la imagen, una subida a catbox
  no se puede eliminar, un servidor `http://` simple la envía sin cifrar. No subas
  contenido sensible a un alojamiento público mediante enlace. Las claves de API de un
  archivo `.sxcu` importado se guardan en `settings.json`, igual que las guarda ShareX.
- **Las copias de traspaso siguen siendo privadas.** Las imágenes que se pasan a un
  cliente de correo o a un editor van a una carpeta que solo tú puedes leer
  (`$XDG_RUNTIME_DIR` en Linux, `%TEMP%\Snipdeck` en Windows) y se eliminan después.
- **Los registros no dejan rastro.** Las coordenadas del puntero y los títulos de
  ventana solo se registran en el nivel de depuración.
- **El arranque automático al iniciar sesión se basa en archivos.** Un acceso directo
  normal en `shell:Startup` en Windows (no la clave `HKCU\…\Run`, la que las herramientas
  de seguridad marcan con más frecuencia) y una entrada de autoarranque XDG en Linux.

---

## 🗺️ Hoja de ruta

- [x] Windows 10 / 11
- [x] **Linux / GNOME** (Wayland) — con la extensión de GNOME Shell incluida: captura
      con `Super`+`Alt`+arrastre o con los atajos, recortes colocados donde se capturaron y
      mantenidos por encima, OCR, el selector de color
- [x] **Linux / KDE Plasma** (Wayland) — con el script de KWin incluido: recortes
      colocados donde se capturaron y mantenidos por encima; captura desde los atajos y la
      barra de herramientas, OCR, el selector de color
- [x] **Linux / X11** (Xfce, MATE, Cinnamon, GNOME en Xorg…) — todo, con los recortes
      mantenidos por encima por el gestor de ventanas
- [~] **Linux / wlroots** (sway, Hyprland, river) — captura desde los atajos y la barra de
      herramientas, OCR, el selector de color; los recortes ni se colocan ni se mantienen
      por encima (ver más abajo)

- [ ] **macOS** — en camino
- [ ] Ventana de ajustes dentro de la aplicación
- [ ] Más herramientas de anotación (desenfoque / pixelado, insignias numeradas de pasos, mover/editar formas)

Fuera de GNOME no hay gesto de modificador + arrastre: requiere un hook global del
puntero, que Linux solo concede leyendo `/dev/input`, es decir, un keylogger de todo el
sistema que Snipdeck no te pedirá que actives.

### Uso en Linux

En Wayland, los atajos globales pasan por el portal del escritorio, que rechaza la sesión
de un cliente sin identificador de aplicación. Abierto desde el menú de aplicaciones,
Snipdeck tiene uno (`com.revisetouch.Snipdeck`, el nombre de su entrada de escritorio);
iniciado desde una terminal, registra él mismo ese identificador, lo que requiere
xdg-desktop-portal 1.19 o posterior; con un portal más antiguo, usa el menú.

Dependencias en tiempo de ejecución: `libayatana-appindicator` (bandeja), `xdg-utils` (compartir)
y `tesseract` junto con los datos de idioma que quieras (OCR, opcional: sin él, el OCR
se omite en lugar de fallar).

**Lo que Wayland no permite.** Un cliente de Wayland no puede colocar sus propias
ventanas, preguntar dónde están ni pedir que se mantengan por encima de las demás; las
tres cosas son competencia del compositor. Así que, en una sesión de Wayland sin más, un recorte
se abre donde lo ponga el compositor en lugar de sobre la región de la que se capturó,
no puede flotar por encima de otras aplicaciones y aparece en la lista de ventanas. Todo
lo demás funciona: la captura de regiones, la superposición congelada, los atajos globales,
el OCR, el arrastre y el menú contextual.

**En KDE Plasma, un script de KWin hace lo mismo** (`packaging/kwin-script/`): mantiene
los recortes por encima de las demás ventanas, los coloca donde se capturaron, los muestra
en todos los escritorios y los saca de la barra de tareas. Se carga sin volver a iniciar
sesión.

En los compositores wlroots (sway, Hyprland, river) todavía no hay nada que lo haga: los
recortes se abren donde los pone el compositor y no se mantienen por encima. Una regla de
ventana en la propia configuración del compositor puede hacerlos flotantes: su título es
`SnipdeckFloating …`.

**En GNOME, una extensión de shell cubre ese hueco.** Lo que un cliente no puede hacer
sí puede hacerlo el código que se ejecuta dentro de GNOME Shell. Por eso `install.sh` también instala
`snipdeck@revisetouch.com` (desde `packaging/gnome-extension/`) cuando detecta una
sesión de GNOME: un único archivo que vigila las ventanas de recorte de Snipdeck y
las coloca donde se capturaron, las mantiene por encima de las demás ventanas, las fija
en todos los espacios de trabajo y las saca del selector de ventanas. La posición
viaja en el título de la ventana, que nunca es visible en estas ventanas sin bordes.

La extensión también recupera el gesto de captura. En GNOME, `Super`+arrastre
ya mueve ventanas, así que el gesto es **`Super`+`Alt`+arrastre**, activo mientras
Snipdeck se está ejecutando:

| Gesto (GNOME) | Resultado |
|---|---|
| `Super`+`Alt` + arrastre | Recorte flotante en pantalla |
| `Super`+`Alt`+`Ctrl` + arrastre | Recorte flotante **y** copia al portapapeles |
| `Super`+`Alt`+`Shift` + arrastre | Solo galería (sin ventana flotante) |
| `Super`+`Alt`+`Ctrl`+`Shift` + arrastre | Galería **y** portapapeles, sin ventana flotante |

«Solo portapapeles» (`Win`+`Alt` en Windows) no tiene gesto aquí; usa su
atajo. `Esc` cancela un arrastre, y mantener pulsadas `Super`+`Alt` te permite hacer
varios recortes seguidos.

Cierra la sesión y vuelve a entrar después de instalar; GNOME Shell no puede recargar
extensiones en caliente en Wayland, porque *es* el compositor que sostiene tu sesión. La
extensión es totalmente opcional (sin ella, Snipdeck se comporta como cualquier otro
cliente de Wayland) y, aparte de las propias ventanas de Snipdeck, solo afecta a
`Super`+`Alt`+arrastre, y solo mientras Snipdeck se está ejecutando.

Lo que aún falta en GNOME Wayland: un recorte que arrastras lo mueve Mutter, y la
aplicación nunca llega a saber dónde quedó, así que su posición no se recuerda entre
reinicios.

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
| `upload_service` | `"catbox"` | Alojamiento de imágenes para *Subir*: `"catbox"` (sin clave), `"imgur"` o `"custom:<name>"`; se elige desde `⋯` → Subir a |
| `upload_hosts` | `[]` | Alojamientos importados de archivos `.sxcu` de ShareX, con los nombres de campo del propio ShareX |
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
| `src/sxcu.rs`, `src/upload_window.rs`, `ui/upload.slint` | Subidas personalizadas de ShareX; la ventana de consentimiento / progreso / resultado de la subida |
| `src/tray.rs`, `src/autostart.rs`, `src/single_instance.rs` | Icono de la bandeja, arranque al iniciar sesión, protección de instancia única |
| `src/snip.rs`, `src/settings.rs`, `src/paths.rs` | Persistencia de la galería (SQLite + FTS), ajustes, rutas |
| `src/image_files.rs`, `src/atomic_file.rs` | Imágenes de los recortes en disco: WebP sin pérdida, escrituras atómicas |
| `src/i18n.rs`, `lang/` | i18n en tiempo de ejecución + catálogos `.po` de gettext integrados (23 idiomas) |

</details>

---

## ✨ Hecho por ReviseTouch

Snipdeck es una aplicación gratuita y de código abierto de **[ReviseTouch](https://revisetouch.com)**:
creamos herramientas para desarrolladores y productos de IA. Descubre qué más estamos creando en
**[revisetouch.com](https://revisetouch.com)**.

---

## Licencia

[MIT](LICENSE) © 2026 Hasan Salihoğlu

<div align="center">
<sub>Construido con 🦀 Rust y <a href="https://slint.dev">Slint</a>.</sub>
</div>
