<div align="center">

[English](README.md) · [Türkçe](README.tr.md) · [Español](README.es.md) · [简体中文](README.zh-CN.md) · [العربية](README.ar.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · [日本語](README.ja.md) · [Français](README.fr.md) · **Português**

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**Um _deck_ de recortes de tela rápido e controlado pelo teclado para Windows.**

Capture uma região, mantenha os recortes flutuando sobre tudo, faça anotações neles,
indexe-os por OCR e pesquise centenas de capturas de tela em uma única galeria densa —
tudo a partir de um aplicativo Rust nativo com interface [Slint](https://slint.dev) renderizada por software.

![platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows&logoColor=white)
![soon](https://img.shields.io/badge/macOS%20%2F%20Linux-on%20the%20way-lightgrey)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Slint-orange?logo=rust&logoColor=white)
![languages](https://img.shields.io/badge/languages-23-success)
![license](https://img.shields.io/badge/license-MIT-green)

</div>

---

## 📸 Capturas de tela

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · translate.png
-->

<div align="center">

| O espaço de trabalho da galeria | Recortes flutuantes na área de trabalho |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="Galeria" width="420" /> | <img src="docs/screenshots/floating.png" alt="Recortes flutuantes" width="420" /> |
| **O editor de anotações** | **OCR + tradução** |
| <img src="docs/screenshots/annotate.png" alt="Editor de anotações" width="420" /> | <img src="docs/screenshots/translate.png" alt="Tradução por OCR" width="420" /> |

</div>

---

## ✨ Destaques

- 🖱️ **Capture com um modificador + arrastar.** Segure `Win` e arraste uma região; os modificadores decidem o que
  acontece com o resultado — janela flutuante, área de transferência, somente galeria (matriz abaixo).
- 🧊 **Captura com congelamento primeiro** (`Win`+`Shift`+`Space`). Escurece e congela a tela *antes* de você
  selecionar, para que dicas de ferramenta exibidas ao passar o mouse e outras interfaces sensíveis a cliques permaneçam capturadas.
- 📌 **Recortes flutuantes.** As capturas ficam fixadas no topo como sobreposições leves e sem bordas que você pode
  arrastar, redimensionar, fixar, recortar, girar/inverter e descartar.
- ✏️ **Editor de anotações integrado.** Caneta, marca-texto, formas, setas e texto com uma roda de cores HSV
  completa, tamanho + opacidade ajustáveis, zoom e desfazer/refazer (detalhes).
- 🔍 **OCR integrado.** Cada recorte é indexado por OCR em segundo plano com o mecanismo do sistema operacional
  (`Windows.Media.Ocr`) — sem download de modelo, sem rede, totalmente local.
- 🌐 **OCR + tradução.** Leia o texto de um recorte em outro idioma diretamente no app (mais de 30 idiomas de destino, com
  favoritos que você pode marcar com estrela).
- 🗂️ **Galeria poderosa.** Pesquisa de texto completo no texto reconhecido por OCR, metadados da janela, monitor e caminho;
  filtre por status / conteúdo / tamanho / data; uma grade virtualizada que permanece fluida com milhares de recortes.
- 📤 **Compartilhar e enviar.** Painel de compartilhamento do Windows, um novo e-mail (Simple MAPI), o editor de imagens do sistema
  ou um servidor de hospedagem de imagens — os envios são **opcionais, com uma caixa de diálogo de consentimento explícita**.
- 🌍 **23 idiomas**, detectados automaticamente a partir do seu sistema operacional e alternáveis pela barra de ferramentas.
- 🌗 **Temas claro / escuro**, uma base na bandeja do sistema e início ao fazer login **sem registro**.

---

## ⌨️ Atalhos de captura

Segure `Win` e, em seguida, arraste uma região com o botão **esquerdo** do mouse. Adicione modificadores para alterar o que
acontece com o recorte:

| Atalho | Resultado |
| --- | --- |
| `Win` + arrastar | Recorte flutuante na tela |
| `Win`+`Ctrl` + arrastar | Recorte flutuante **e** cópia para a área de transferência |
| `Win`+`Alt` + arrastar | Somente área de transferência (não adicionado à galeria) |
| `Win`+`Shift` + arrastar | Somente galeria (sem janela flutuante) |
| `Win`+`Ctrl`+`Shift` + arrastar | Galeria **e** área de transferência, sem janela flutuante |
| `Win`+`Shift`+`Space`, depois arrastar | **Congelamento primeiro** — congela a tela e depois seleciona na imagem estática |

Você também pode clicar com o botão direito no ícone da bandeja → **Novo Recorte** para preparar uma captura de arrastar única sem
segurar `Win`.

---

## ✏️ Editor de anotações

Clique com o botão direito em qualquer recorte → **Anotar** para abrir um editor focado, no estilo Telegram, que fica sobre
os seus recortes.

- **Ferramentas** — caneta de mão livre, marca-texto, retângulo, elipse, linha, seta e texto.
- **Roda de cores HSV completa** com um controle deslizante de brilho — escolha *qualquer* cor, não uma paleta fixa.
- **Controles deslizantes de tamanho e opacidade** com uma prévia piscante do tamanho sobre a imagem, lembrada por ferramenta.
- **Aproximar / afastar / ajustar zoom** para trabalho preciso; a tela se move quando ampliada.
- **Desfazer / refazer / limpar**, além de `Esc`, `Ctrl`+`Z`, `Ctrl`+`Y`.
- Ao **Salvar**, os traços são achatados sobre a imagem em resolução total (nítidos, em tamanho nativo).

A prévia em tempo real e o resultado salvo vêm do mesmo código, então **o que você vê é o que você obtém**.

---

## 🖱️ Ações do botão direito

Clique com o botão direito em um recorte flutuante **ou** em uma miniatura da galeria:

- **Copiar** a imagem para a área de transferência
- **OCR + Área de transferência** — executa o OCR e copia o texto reconhecido
- **OCR + Traduzir** — abre a janela de tradução
- **Anotar** — abre o editor acima
- **Compartilhar** — folha de compartilhamento do Windows, **E-mail** (anexo MAPI), abrir no **editor** (Paint),
  ou **Enviar** para um servidor de hospedagem de imagens
- **Transformar** — girar (90° / 180°), inverter horizontal / vertical
- **Recortar** (arraste as alças para dentro) e **Redimensionar**
- **Ocultar da tela** / **Excluir**

---

## 📦 Instalação

### Download

Baixe o `snipdeck.exe` mais recente na página de [**Releases**](https://github.com/hasantr/snipdeck/releases).
É um único executável portátil — sem instalador.

### Compilar a partir do código-fonte

Requisitos: [Rust](https://rustup.rs/) (estável) e Windows 10 ou 11.

```powershell
git clone https://github.com/hasantr/snipdeck.git
cd snipdeck
cargo build --release
```

O binário é gerado em `target\release\snipdeck.exe`.

---

## 🌍 Idiomas

O Snipdeck é totalmente localizado. No primeiro início, ele escolhe o seu idioma a partir da localidade do sistema operacional;
substitua-o a qualquer momento pelo **menu do globo** na barra de ferramentas.

> Inglês · Espanhol · Chinês Simplificado · Híndi · Árabe · Português · Russo · Japonês ·
> Alemão · Francês · Coreano · Italiano · Turco · Indonésio · Vietnamita · Polonês · Ucraniano ·
> Holandês · Persa · Tailandês · Bengali · Urdu · Romeno

Todo o aplicativo — tanto a interface Slint (`@tr`) quanto as mensagens do lado Rust (`i18n::t`) — lê de
**um único arquivo gettext `.po` por idioma** em `lang/<code>/LC_MESSAGES/snipdeck.po`, empacotado no
binário no momento da compilação (sem dependência de gettext em tempo de execução).

<details>
<summary><b>Adicionar ou atualizar uma tradução</b></summary>

1. Novas strings são envolvidas em `@tr("…")` (em `.slint`) ou `i18n::t("…")` (em Rust).
2. `lang/snipdeck.pot` é o modelo que lista todas as strings em inglês.
3. Copie um novo msgid para cada `lang/<code>/LC_MESSAGES/snipdeck.po` e preencha seu `msgstr` —
   mantenha os marcadores de posição (`{}`, `{e}`, `{url}`, `{path}`) e os escapes `\r\n` intactos.
4. Para adicionar um novo idioma, crie seu `.po` e adicione o código em `LANGUAGES` e `po_source()`
   em `src/i18n.rs`.

</details>

---

## 🔒 Privacidade e segurança

- **A captura e o OCR são totalmente locais.** O OCR usa o mecanismo integrado `Windows.Media.Ocr`; nada
  é enviado para qualquer lugar durante a captura ou a indexação.
- **O início ao fazer login é sem registro.** O Snipdeck coloca um atalho normal na sua pasta `shell:Startup`
  em vez de gravar um valor `HKCU\…\Run` — o mecanismo comum e visível ao usuário que
  evita a chave de registro de início automático que ferramentas de antivírus / EDR mais frequentemente sinalizam.
- **Os envios são opcionais.** Nada sai da sua máquina sem uma caixa de diálogo de consentimento explícita que
  especifica o destino. Não envie conteúdo sensível para um servidor público por link.
- **O OCR + Traduzir usa a rede.** Apenas o *texto* reconhecido por OCR é enviado ao endpoint público
  `translate_a` do Google para ser traduzido; ele não é usado em nenhum outro lugar.

---

## 🗺️ Roteiro

- [x] Windows 10 / 11
- [ ] **macOS** — a caminho
- [ ] **Linux** — a caminho
- [ ] Janela de configurações dentro do app
- [ ] Mais ferramentas de anotação (desfoque / pixelizar, emblemas de etapa numerados, mover/editar formas)

> O Snipdeck é exclusivo para Windows hoje porque a captura, o OCR, a bandeja e o compartilhamento usam
> APIs específicas do Windows. A interface ([Slint](https://slint.dev)) e a maior parte do núcleo já são
> multiplataforma, então **as compilações para macOS e Linux estão a caminho.**

---

## ⚙️ Configuração

As configurações são armazenadas como JSON e criadas na primeira execução. Ainda não há uma janela de configurações — edite o
arquivo diretamente e, em seguida, reinicie o Snipdeck.

| Caminho | Conteúdo |
| --- | --- |
| `%APPDATA%\Snipdeck\settings.json` | Configurações do usuário (abaixo) |
| `%LOCALAPPDATA%\Snipdeck\` | `snips.db`, `state.json` e o `cache\` de imagens + miniaturas |
| `Pictures\Snipdeck\` | Pasta padrão para *Salvar* |

<details>
<summary><b>Chaves selecionadas de <code>settings.json</code></b></summary>

| Chave | Padrão | Significado |
| --- | --- | --- |
| `language` | `"auto"` | Código do idioma da interface, ou `"auto"` para seguir o sistema operacional |
| `dark_theme` | `true` | Aparência clara ou escura |
| `upload_service` | `"catbox"` | Servidor de hospedagem de imagens para *Enviar*: `"catbox"` (sem chave) ou `"imgur"` |
| `imgur_client_id` | `""` | Client-ID do Imgur, necessário apenas quando `upload_service` for `"imgur"` |
| `translate_target` | `"en"` | Código do idioma de destino padrão para a tradução por OCR |
| `translate_favs` | `["en","es","fr","de","it","pt","ru","ar"]` | Idiomas de seleção rápida na janela de tradução |
| `default_format` | `"png"` | Extensão padrão para *Salvar* |

**Serviços de envio** — o *catbox.moe* (padrão) não precisa de conta ou chave, mas os envios anônimos
**não podem ser excluídos por você** posteriormente. O *Imgur* precisa de um Client-ID gratuito (registre-se uma vez em
imgur.com → Settings → Applications). Em ambos os casos, a imagem fica não listada, mas qualquer pessoa com o
link pode abri-la, e ela não é criptografada.

</details>

---

## 🧩 Arquitetura

Um loop de eventos Slint de thread único controla a interface; os hooks do sistema operacional e o trabalho bloqueante são executados em suas próprias
threads e devolvem os resultados por canais, esvaziados uma vez por tick.

<details>
<summary><b>Mapa de módulos</b></summary>

| Módulo | Responsabilidade |
| --- | --- |
| `src/app/` | Estado do app, o loop por tick, ações da interface, modelo da galeria |
| `src/capture.rs`, `src/dxgi_capture.rs` | Captura do monitor (DXGI Desktop Duplication, fallback GDI) |
| `src/mouse_trigger.rs`, `src/native_mouse_trigger.rs` | Hooks de mouse/teclado de baixo nível que preparam e controlam os recortes |
| `src/slint_overlay.rs`, `ui/overlay.slint` | Sobreposição de seleção em tela cheia (ao vivo ou congelada) |
| `src/floating.rs`, `ui/floating.slint` | Janelas de recortes flutuantes (arrastar, redimensionar, recortar, borda) |
| `src/annotate.rs`, `ui/annotate.slint` | Editor de anotações (achatamento tiny-skia, roda HSV, zoom) |
| `src/ocr.rs` | Wrapper de `Windows.Media.Ocr` (em uma thread de trabalho) |
| `src/translate.rs`, `src/translate_popup.rs` | Janela de tradução por OCR |
| `src/share.rs`, `src/upload.rs` | Painel de compartilhamento / e-mail MAPI / editor / envio a servidor de imagens |
| `src/tray.rs`, `src/autostart.rs`, `src/single_instance.rs` | Ícone da bandeja, início ao fazer login, proteção de instância única |
| `src/snip.rs`, `src/settings.rs`, `src/paths.rs` | Persistência da galeria (SQLite + FTS), configurações, caminhos |
| `src/i18n.rs`, `lang/` | i18n em tempo de execução + catálogos gettext `.po` empacotados (23 idiomas) |

</details>

---

## Licença

[MIT](LICENSE) © 2026 Hasan Salihoğlu

<div align="center">
<sub>Construído com 🦀 Rust e <a href="https://slint.dev">Slint</a>.</sub>
</div>
