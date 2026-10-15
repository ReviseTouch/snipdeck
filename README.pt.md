<div align="center">

[English](README.md) · [Türkçe](README.tr.md) · [Español](README.es.md) · [简体中文](README.zh-CN.md) · [العربية](README.ar.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · [日本語](README.ja.md) · [Français](README.fr.md) · **Português**

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**Um _deck_ de recortes de tela rápido e controlado pelo teclado para Windows.**

<sub>Um aplicativo gratuito e de código aberto da <a href="https://revisetouch.com"><b>ReviseTouch</b></a>.</sub>

Capture uma região, mantenha os recortes flutuando sobre tudo, faça anotações neles,
indexe-os por OCR e pesquise centenas de capturas de tela em uma única galeria densa —
tudo a partir de um aplicativo Rust nativo com interface [Slint](https://slint.dev) renderizada por software.

![platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows&logoColor=white)
![linux](https://img.shields.io/badge/Linux-GNOME%20full%20%2F%20others%20partial-yellow) ![soon](https://img.shields.io/badge/macOS-on%20the%20way-lightgrey)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Slint-orange?logo=rust&logoColor=white)
![languages](https://img.shields.io/badge/languages-23-success)
![license](https://img.shields.io/badge/license-MIT-green)

</div>

---

## 📸 Capturas de tela

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · collage.png
-->

<div align="center">

| O espaço de trabalho da galeria | Recortes flutuantes na área de trabalho |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="Galeria" width="420" /> | <img src="docs/screenshots/floating.png" alt="Recortes flutuantes" width="420" /> |
| **O editor de anotações** | **O editor de colagens** |
| <img src="docs/screenshots/annotate.png" alt="Editor de anotações" width="420" /> | <img src="docs/screenshots/collage.png" alt="Editor de colagens" width="420" /> |

</div>

---

## ✨ Destaques

- 🖱️ **Capture com um modificador + arrastar.** Segure `Win` e arraste uma região; os modificadores decidem o que
  acontece com o resultado — janela flutuante, área de transferência, somente galeria ([matriz abaixo](#️-atalhos-de-captura)).
- 🧊 **Captura com congelamento primeiro** (`Win`+`Shift`+`Space`). Escurece e congela a tela *antes* de você
  selecionar, para que dicas de ferramenta exibidas ao passar o mouse e outras interfaces sensíveis a cliques permaneçam capturadas.
- 📌 **Recortes flutuantes.** As capturas ficam fixadas no topo como sobreposições leves e sem bordas que você pode
  arrastar, redimensionar, fixar, recortar, girar/inverter e descartar.
- ✏️ **Editor de anotações integrado.** Caneta, marca-texto, formas, setas e texto com uma roda de cores HSV
  completa, tamanho + opacidade ajustáveis, zoom e desfazer/refazer ([detalhes](#️-editor-de-anotações)).
- 🧩 **Colagem.** Organize vários recortes em uma única imagem — grade, linhas justificadas ou mosaico (masonry),
  com espaçamento, cantos arredondados e fundo — salva de volta na galeria como um novo recorte.
- 🔍 **OCR integrado.** Cada recorte é indexado por OCR em segundo plano com o mecanismo do sistema operacional
  (`Windows.Media.Ocr`) — sem download de modelo, sem rede, totalmente local.
- 🌐 **OCR + tradução.** Leia o texto de um recorte em outro idioma diretamente no app (29 idiomas de destino, com
  favoritos que você pode marcar com estrela).
- 🗂️ **Galeria poderosa.** Pesquisa de texto completo no texto reconhecido por OCR, metadados da janela, monitor e caminho;
  filtre por status / conteúdo / tamanho / data; uma grade virtualizada que permanece fluida com milhares de recortes
  ([visita guiada](#️-a-galeria)).
- 📤 **Compartilhar e enviar.** Painel de compartilhamento do Windows, um novo e-mail (Simple MAPI), o editor de imagens do sistema
  ou um servidor de hospedagem de imagens — catbox.moe, Imgur ou **o seu próprio servidor** a partir de um arquivo `.sxcu`
  do ShareX ([detalhes](#-seu-próprio-servidor-de-imagens)). Todo envio **pergunta antes**.
- 🌍 **23 idiomas**, detectados automaticamente a partir do seu sistema operacional e alternáveis pelo menu `⋯` da barra de ferramentas.
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

Você também pode clicar com o botão direito no ícone da bandeja → **Novo recorte** para preparar uma captura de arrastar única sem
segurar `Win`.

### Atalhos globais

Além de `Win` + arrastar, o Snipdeck registra cinco atalhos globais configuráveis:

| Atalho | Ação |
| --- | --- |
| `Win`+`Alt`+`H` | Mostrar / ocultar todos os recortes na tela |
| `Ctrl`+`Alt`+`Shift`+`A` | Novo recorte |
| `Ctrl`+`Alt`+`Shift`+`C` | Novo recorte **e** copiar para a área de transferência |
| `Ctrl`+`Alt`+`Shift`+`X` | Somente área de transferência |
| `Ctrl`+`Alt`+`Shift`+`G` | Abrir a galeria |

Altere-os com as chaves `hotkey_*` em `settings.json`. No Wayland, os atalhos
globais pertencem ao ambiente de desktop: eles são apenas propostos no primeiro início e,
depois, você os altera nas configurações de teclado do seu ambiente de desktop.

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
- **Compartilhar** — painel de compartilhamento do Windows, **E-mail** (anexo MAPI), abrir no **editor** (Paint),
  ou **Enviar** para um servidor de hospedagem de imagens ([qual](#-seu-próprio-servidor-de-imagens))
- **Transformar** — girar (90° / 180°), inverter horizontal / vertical
- **Recortar** (arraste as alças para dentro) e **Redimensionar**
- **Ocultar da tela** / **Excluir**

---

## 🗂️ A galeria

Uma única linha na barra de ferramentas; todo o resto está a um clique.

- **Novo** faz um recorte. A seta ao lado guarda **Recortar + Copiar** e **Só copiar**, cada um com seu atalho.
- **Colagem** e **Escolher cor** ficam ao lado, e a caixa de pesquisa ocupa o resto da linha —
  `Ctrl`+`F` para focá-la, `Esc` para limpá-la.
- **Filtrar** reúne status, conteúdo, tamanho e data em um único painel. Os filtros ativos aparecem como etiquetas
  abaixo da barra de ferramentas, e cada um pode ser removido individualmente ou todos de uma vez com **Limpar filtros**.
- **`⋯`** guarda o que muda raramente: recortes flutuantes na barra de tarefas, tema escuro, iniciar ao
  entrar, idioma, limpar a galeria e Sobre o Snipdeck.
- A barra de status conta o que está sendo exibido e contém **Ordenar** e o tamanho dos cartões **S / M / L**.

Passe o mouse sobre um cartão para ver as ações rápidas; clique nele com o botão direito para [todo o resto](#️-ações-do-botão-direito).

---

## 📤 Seu próprio servidor de imagens

**`⋯` → Enviar para** escolhe para onde **Enviar** manda um recorte: catbox.moe (o padrão, sem conta),
o Imgur ou um servidor seu. Para adicionar um, escolha **Adicionar de um arquivo .sxcu…** e selecione um arquivo
de envio personalizado do ShareX — o formato que Zipline, XBackBone, Chevereto e a maioria dos servidores de
imagens auto-hospedados já oferecem, então não há nada para escrever à mão.

O Snipdeck envia a imagem como dados de formulário multipart ou como corpo binário, com os cabeçalhos, argumentos
e parâmetros de consulta do arquivo, e lê o link com `{json:…}`, `{regex:…}`, `{header:…}` ou `{response}` —
também na forma `$json:…$` do ShareX 12. Um arquivo que precise de mais (uma solicitação de entrada, um valor
aleatório, uma resposta XML, um destino que não seja um servidor de imagens) é recusado já na importação, com o
motivo, em vez de falhar no primeiro envio.

Antes de qualquer envio, a própria janela do Snipdeck mostra o servidor e o seu endereço e diz o que o envio
significa; ao terminar, mostra o link, que já foi copiado.

---

## 📦 Instalação

### Download

**Windows** — baixe o `snipdeck-windows-x64.exe` na página de
[**Releases**](https://github.com/ReviseTouch/snipdeck/releases). Um único
executável portátil, sem instalador.

**Linux (x64)** — na mesma página, o pacote da sua distribuição:

```bash
sudo apt install ./snipdeck_*_amd64.deb      # Debian, Ubuntu, Mint
sudo dnf install ./snipdeck-*.x86_64.rpm     # Fedora, RHEL, Rocky
sudo zypper install ./snipdeck-*.x86_64.rpm  # openSUSE
```

ou, em qualquer distribuição e sem root, o tarball:

```bash
tar xzf snipdeck-linux-x64.tar.gz
cd snipdeck-linux-x64
./install.sh          # into ~/.local; --uninstall reverses it
```

Os pacotes funcionam em qualquer distribuição com glibc 2.30 ou mais recente — Debian 11,
Ubuntu 20.04, RHEL 9 e tudo o que veio depois — e foram instalados e iniciados no Debian
12 e 13, Ubuntu 22.04 e 24.04, Fedora, Rocky 9, openSUSE Leap 15.6 e Tumbleweed
(`packaging/linux/test-install.sh`). No GNOME e no KDE Plasma, eles também instalam o
auxiliar de desktop do Snipdeck (veja abaixo), que o Snipdeck ativa na primeira execução;
o GNOME carrega uma extensão recém-instalada no próximo login.

### Compilar a partir do código-fonte

Requisitos: [Rust](https://rustup.rs/) (estável).

```bash
git clone https://github.com/ReviseTouch/snipdeck.git
cd snipdeck
cargo build --release
```

**Windows** (10 ou 11): o binário é gerado em `target\release\snipdeck.exe`.

**Linux**: a compilação precisa dos cabeçalhos de GTK, X11/Wayland, bandeja do sistema, D-Bus e GL.
No Debian/Ubuntu:

```bash
sudo apt install libgtk-3-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxcb-randr0-dev libxrandr-dev libxkbcommon-dev \
  libwayland-dev libayatana-appindicator3-dev libfontconfig-dev libdbus-1-dev \
  libgl1-mesa-dev libegl1-mesa-dev
```

O PipeWire não precisa de nada no momento da compilação: ele é carregado em tempo de execução
para a captura no Wayland. Para instalar sua compilação do mesmo jeito que uma versão
publicada é instalada, coloque-a ao lado do instalador:

```bash
packaging/linux/install-from-source.sh   # builds, stages afresh, installs
```

---

## 🌍 Idiomas

O Snipdeck é totalmente localizado. No primeiro início, ele escolhe o seu idioma a partir da localidade do sistema operacional;
substitua-o a qualquer momento em **`⋯` → Idioma** na barra de ferramentas.

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
5. Execute `tools/i18n-check.py`: ele verifica se cada texto está traduzido, com os marcadores intactos; a CI também o executa.

</details>

---

## 🔒 Privacidade e segurança

- **A captura e o OCR são locais.** O OCR usa o `Windows.Media.Ocr` no Windows e o
  Tesseract no Linux (alimentado por um pipe, sem arquivo temporário); nada é enviado
  para lugar nenhum durante a captura ou a indexação. No Wayland, a tela é lida pelo
  portal ScreenCast: você concede a permissão uma vez, e o indicador de compartilhamento
  aparece apenas no momento em que um recorte é feito.
- **O OCR + Traduzir pergunta antes.** Na primeira vez, ele explica que o *texto*
  reconhecido por OCR — nunca a imagem — vai para o Google Tradutor, e espera o seu sim.
  O texto é enviado no corpo da requisição, não na URL.
- **Os envios perguntam antes, sempre.** Nada sai da sua máquina até você clicar em
  **Enviar** em uma janela que mostra o servidor e o seu endereço e diz o que isso
  significa: qualquer pessoa com o link pode abrir a imagem, um envio ao catbox não
  pode ser excluído, um servidor `http://` simples a envia sem criptografia. Não envie
  conteúdo sensível para um servidor público por link. As chaves de API de um arquivo
  `.sxcu` importado ficam em `settings.json`, como o ShareX as guarda.
- **As cópias repassadas continuam privadas.** As imagens passadas a um cliente de e-mail
  ou a um editor vão para uma pasta que só você pode ler (`$XDG_RUNTIME_DIR` no Linux,
  `%TEMP%\Snipdeck` no Windows) e são apagadas depois.
- **Os logs não guardam rastros.** As coordenadas do ponteiro e os títulos das janelas
  só são registrados no nível de depuração.
- **O início ao fazer login é baseado em arquivo.** Um atalho normal em `shell:Startup` no
  Windows (e não a chave `HKCU\…\Run`, que as ferramentas de segurança mais costumam sinalizar),
  uma entrada de inicialização automática XDG no Linux.

---

## 🗺️ Roteiro

- [x] Windows 10 / 11
- [x] **Linux / GNOME** (Wayland) — com a extensão do shell incluída: captura
      com `Super`+`Alt`+arrastar ou pelos atalhos, recortes posicionados onde foram capturados
      e mantidos no topo, OCR, o seletor de cores
- [x] **Linux / KDE Plasma** (Wayland) — com o script do KWin incluído: recortes
      posicionados onde foram capturados e mantidos no topo; captura pelos atalhos e pela
      barra de ferramentas, OCR, o seletor de cores
- [x] **Linux / X11** (Xfce, MATE, Cinnamon, GNOME no Xorg…) — tudo, com os recortes
      mantidos no topo pelo gerenciador de janelas
- [~] **Linux / wlroots** (sway, Hyprland, river) — captura pelos atalhos e pela barra de
      ferramentas, OCR, o seletor de cores; os recortes não são posicionados nem mantidos
      no topo (veja abaixo)

- [ ] **macOS** — a caminho
- [ ] Janela de configurações dentro do app
- [ ] Mais ferramentas de anotação (desfoque / pixelizar, emblemas de etapa numerados, mover/editar formas)

Não há gesto de modificador+arrastar fora do GNOME: ele exige um hook global do ponteiro,
que o Linux só concede pela leitura de `/dev/input` — um keylogger de todo o sistema que o
Snipdeck não vai pedir para você ativar.

### Executando no Linux

No Wayland, os atalhos globais passam pelo portal do desktop, que recusa a sessão de um
cliente sem ID de aplicativo. Aberto pelo menu de aplicativos, o Snipdeck tem um
(`com.revisetouch.Snipdeck`, o nome da sua entrada de desktop); iniciado por um terminal,
ele mesmo registra esse ID, o que exige o xdg-desktop-portal 1.19 ou mais recente — com um
portal mais antigo, use o menu.

Dependências de tempo de execução: `libayatana-appindicator` (bandeja), `xdg-utils` (compartilhamento)
e `tesseract` mais os dados de idioma que você quiser (OCR — opcional; sem ele, o OCR
é ignorado em vez de falhar).

**O que o Wayland não permite.** Um cliente Wayland não pode posicionar as próprias
janelas, perguntar onde elas estão nem pedir que fiquem acima das outras; o
compositor controla as três coisas. Assim, em uma sessão Wayland pura, um recorte abre onde o
compositor o colocar, e não sobre a região de onde foi capturado, não consegue
flutuar acima de outros aplicativos e aparece na lista de janelas. Todo o
resto — captura de região, a sobreposição congelada, atalhos globais, OCR, arrastar, o
menu de contexto — funciona.

**No KDE Plasma, um script do KWin faz o mesmo** (`packaging/kwin-script/`): ele mantém os
recortes acima das outras janelas, posiciona-os onde foram capturados, coloca-os em todas
as áreas de trabalho e os mantém fora da barra de tarefas. Ele é carregado sem um novo
login.

Nos compositores wlroots (sway, Hyprland, river), nada faz isso ainda: os recortes abrem
onde o compositor os coloca e não ficam no topo. Uma regra de janela na própria
configuração do compositor pode deixá-los flutuantes — o título deles é
`SnipdeckFloating …`.

**No GNOME, uma extensão do shell fecha a lacuna.** O que um cliente não pode fazer, um código
executado dentro do GNOME Shell pode. Por isso, o `install.sh` também instala
`snipdeck@revisetouch.com` (de `packaging/gnome-extension/`) quando detecta uma
sessão GNOME — um único arquivo que observa as janelas de recorte do Snipdeck e as
posiciona onde foram capturadas, mantém-nas acima das outras janelas, fixa-as em
todos os espaços de trabalho e as remove do alternador de janelas. A posição
viaja no título da janela, que nunca fica visível nessas janelas sem bordas.

A extensão também traz de volta o gesto de captura. No GNOME, `Super`+arrastar
já move janelas, então o gesto é **`Super`+`Alt`+arrastar**, ativo enquanto o
Snipdeck estiver em execução:

| Gesto (GNOME) | Resultado |
|---|---|
| `Super`+`Alt` + arrastar | Recorte flutuante na tela |
| `Super`+`Alt`+`Ctrl` + arrastar | Recorte flutuante **e** cópia para a área de transferência |
| `Super`+`Alt`+`Shift` + arrastar | Somente galeria (sem janela flutuante) |
| `Super`+`Alt`+`Ctrl`+`Shift` + arrastar | Galeria **e** área de transferência, sem janela flutuante |

"Somente área de transferência" — `Win`+`Alt` no Windows — não tem gesto aqui; use o
atalho correspondente. `Esc` cancela um arrasto, e manter `Super`+`Alt` pressionadas permite fazer
vários recortes seguidos.

Saia da sessão e entre novamente após a instalação; o GNOME Shell não consegue recarregar
extensões no lugar no Wayland, porque ele *é* o compositor que mantém a sua sessão. A
extensão é totalmente opcional — sem ela, o Snipdeck se comporta como qualquer outro
cliente Wayland — e, além das próprias janelas do Snipdeck, ela só interfere em
`Super`+`Alt`+arrastar, e apenas enquanto o Snipdeck estiver em execução.

Ainda falta no GNOME Wayland: um recorte que você arrasta é movido pelo Mutter, e o aplicativo
nunca é informado de onde ele parou, então sua posição não é lembrada entre
reinicializações.

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
| `upload_service` | `"catbox"` | Servidor de hospedagem de imagens para *Enviar*: `"catbox"` (sem chave), `"imgur"` ou `"custom:<name>"`; definido em `⋯` → Enviar para |
| `upload_hosts` | `[]` | Servidores importados de arquivos `.sxcu` do ShareX, com os nomes de campo do próprio ShareX |
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
| `src/sxcu.rs`, `src/upload_window.rs`, `ui/upload.slint` | Envios personalizados do ShareX; a janela de consentimento / progresso / resultado do envio |
| `src/tray.rs`, `src/autostart.rs`, `src/single_instance.rs` | Ícone da bandeja, início ao fazer login, proteção de instância única |
| `src/snip.rs`, `src/settings.rs`, `src/paths.rs` | Persistência da galeria (SQLite + FTS), configurações, caminhos |
| `src/image_files.rs`, `src/atomic_file.rs` | Imagens dos recortes no disco: WebP sem perdas, gravações atômicas |
| `src/i18n.rs`, `lang/` | i18n em tempo de execução + catálogos gettext `.po` empacotados (23 idiomas) |

</details>

---

## ✨ Feito pela ReviseTouch

O Snipdeck é um aplicativo gratuito e de código aberto da **[ReviseTouch](https://revisetouch.com)** —
criamos ferramentas para desenvolvedores e produtos de IA. Veja o que mais estamos criando em
**[revisetouch.com](https://revisetouch.com)**.

---

## Licença

[MIT](LICENSE) © 2026 Hasan Salihoğlu

<div align="center">
<sub>Construído com 🦀 Rust e <a href="https://slint.dev">Slint</a>.</sub>
</div>
