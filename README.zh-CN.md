<div align="center">

[English](README.md) · [Türkçe](README.tr.md) · [Español](README.es.md) · **简体中文** · [العربية](README.ar.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · [日本語](README.ja.md) · [Français](README.fr.md) · [Português](README.pt.md)

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**一款快速、以键盘为核心的 Windows 和 Linux 屏幕截图 _工作台_。**

<sub>由 <a href="https://revisetouch.com"><b>ReviseTouch</b></a> 出品的免费开源应用。</sub>

框选一块区域，让截图悬浮于一切窗口之上，对其进行标注、
建立 OCR 索引，并在一个高密度图库中检索数百张截图——
这一切都来自一个原生 Rust 应用，搭配 [Slint](https://slint.dev) 界面。

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

## 📸 截图

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · collage.png
-->

<div align="center">

| 图库工作区 | 桌面上的悬浮截图 |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="图库" width="420" /> | <img src="docs/screenshots/floating.png" alt="悬浮截图" width="420" /> |
| **标注编辑器** | **拼图编辑器** |
| <img src="docs/screenshots/annotate.png" alt="标注编辑器" width="420" /> | <img src="docs/screenshots/collage.png" alt="拼图编辑器" width="420" /> |

</div>

---

## ✨ 亮点

- 🖱️ **按住修饰键 + 拖动即可截图。** 按住 `Win` 并拖动框选一块区域；由修饰键决定
  结果如何处理——悬浮窗口、剪贴板或仅入图库（[见下方矩阵](#️-截图快捷键)）。
- 🧊 **冻屏优先截图**（`Win`+`Shift`+`Space`）。在你选择*之前*先将屏幕变暗并冻结，
  这样悬停提示及其他对点击敏感的界面都能被截下。
- 📌 **悬浮截图。** 截图会作为轻量、无边框的覆盖层固定在最上层，你可以
  拖动、缩放、固定、裁剪、旋转/翻转以及关闭它们。
- ✏️ **内置标注编辑器。** 画笔、荧光笔、形状、箭头和文字，配有完整的 HSV
  色轮、可调节的大小 + 不透明度、缩放以及撤销/重做（[详情](#️-标注编辑器)）。
- 🧩 **拼图。** 将多张截图排成一张图片——网格、两端对齐的行或瀑布流，可设置间距、
  圆角和背景——并作为一张新截图存回图库。
- 🔍 **内置 OCR。** 每张截图都会在后台用操作系统引擎
  （`Windows.Media.Ocr`）建立 OCR 索引——无需下载模型、无需联网，完全本地运行。
- 🌐 **OCR + 翻译。** 内联读取截图中的文字并译为另一种语言（29 种目标语言，
  可标星收藏常用项）。
- 🗂️ **强大的图库。** 对 OCR 文本、窗口元数据、显示器和路径进行全文检索；
  按状态 / 内容 / 尺寸 / 日期筛选；虚拟化网格在数千张截图下依然流畅
  （[导览](#️-图库)）。
- 📤 **分享与上传。** Windows 分享面板、新建邮件（Simple MAPI）、系统图像编辑器，
  或图床——catbox.moe、Imgur，或通过 ShareX `.sxcu` 文件接入的**你自己的服务器**
  （[详情](#-使用自己的图床)）。每次上传都会**先征求同意**。
- 🌍 **23 种语言**，根据你的操作系统自动检测，并可从工具栏的 `⋯` 菜单切换。
- 🪶 **轻量不占资源。** 使用 Rust 编写的原生代码，搭配 [Slint](https://slint.dev) 界面——没有浏览器引擎，也无需安装运行时。内存占用约 68 MB，在托盘中待命时几乎不占用 CPU（[测量数据](#-资源占用)）。
- 🌗 **浅色 / 深色主题**、系统托盘驻留，以及**无注册表**的开机自启动。

---

## ⌨️ 截图快捷键

按住 `Win`，然后用鼠标**左键**拖动框选一块区域。添加修饰键可改变
截图的处理方式：

| 快捷键 | 结果 |
| --- | --- |
| `Win` + 拖动 | 屏幕上的悬浮截图 |
| `Win`+`Ctrl` + 拖动 | 悬浮截图**并**复制到剪贴板 |
| `Win`+`Alt` + 拖动 | 仅剪贴板（不加入图库） |
| `Win`+`Shift` + 拖动 | 仅图库（无悬浮窗口） |
| `Win`+`Ctrl`+`Shift` + 拖动 | 图库**与**剪贴板，无悬浮窗口 |
| `Win`+`Shift`+`Space`，然后拖动 | **冻屏优先**——先冻结屏幕，再在静止画面上选择 |

你也可以右键单击托盘图标 →**新建截图**，无需按住 `Win` 即可预备一次性的拖动截图。

### 全局热键

除了 `Win` + 拖动之外，Snipdeck 还注册了五个可配置的全局热键：

| 热键 | 操作 |
| --- | --- |
| `Win`+`Alt`+`H` | 显示 / 隐藏屏幕上的所有截图 |
| `Ctrl`+`Alt`+`Shift`+`A` | 新建截图 |
| `Ctrl`+`Alt`+`Shift`+`C` | 新建截图**并**复制到剪贴板 |
| `Ctrl`+`Alt`+`Shift`+`X` | 仅剪贴板 |
| `Ctrl`+`Alt`+`Shift`+`G` | 打开图库 |

在 `settings.json` 中通过 `hotkey_*` 键来更改它们。在 Wayland 上，
全局快捷键由桌面环境掌管：这些热键仅在首次启动时提出，
之后需在桌面的键盘设置中更改。

---

## ✏️ 标注编辑器

右键单击任意截图 →**标注**，打开一个专注的、Telegram 风格的编辑器，它会悬浮在
你的截图之上。

- **工具**——自由画笔、荧光笔、矩形、椭圆、直线、箭头和文字。
- **完整的 HSV 色轮**配亮度滑块——可选取*任意*颜色，而非固定调色板。
- **大小与不透明度滑块**，配有在图像上闪烁的大小预览，并按工具分别记忆。
- **放大 / 缩小 / 适配**以便精细操作；放大时画布可平移。
- **撤销 / 重做 / 清除**，外加 `Esc`、`Ctrl`+`Z`、`Ctrl`+`Y`。
- 点击**保存**时，笔迹会以全分辨率拼合到图像上（清晰、原生尺寸）。

实时预览与保存结果出自同一段代码，因此**所见即所得**。

---

## 🖱️ 右键操作

右键单击一张悬浮截图**或**一张图库缩略图：

- **复制**图像到剪贴板
- **OCR + 剪贴板**——运行 OCR 并复制识别出的文字
- **OCR + 翻译**——打开翻译弹窗
- **标注**——打开上述编辑器
- **分享**——Windows 分享面板、**邮件**（MAPI 附件）、在**编辑器**中打开（画图），
  或**上传**到图床（[上传到哪个图床](#-使用自己的图床)）
- **变换**——旋转（90° / 180°）、水平 / 垂直翻转
- **裁剪**（向内拖动手柄）和**调整大小**
- **从屏幕隐藏** / **删除**

---

## 🗂️ 图库

工具栏只有一行；其余一切都只需一次点击。

- **新建**用于截图。它的箭头里有**截图+复制**和**仅复制**，各自带有热键。
- 旁边是**拼图**和**取色**，搜索框占据该行其余部分——
  `Ctrl`+`F` 聚焦，`Esc` 清空。
- **筛选**在一个面板中打开状态、内容、尺寸和日期。生效的筛选条件会以标签形式显示在
  工具栏下方，可逐个移除，也可通过**清除筛选**一次性全部移除。
- **`⋯`** 收纳不常更改的项目：“在任务栏显示悬浮截图”、“深色主题”、“开机自动启动”、
  “语言”、清空图库，以及“关于 Snipdeck”。
- 状态栏统计当前显示的数量，并提供**排序**和 **S / M / L** 卡片尺寸。

将鼠标悬停在卡片上可看到其快捷操作；右键单击则可使用[其他所有操作](#️-右键操作)。

---

## 📤 使用自己的图床

**`⋯` → 上传到** 用于选择 **上传** 把截图发送到哪里：catbox.moe（默认，无需账户）、Imgur，
或你自己的图床。要添加一个，选择 **从 .sxcu 文件添加…**，然后选取一个 ShareX 自定义上传器文件——
Zipline、XBackBone、Chevereto 以及大多数自建图床服务器本就提供这种格式，因此无需手写任何内容。

Snipdeck 以 multipart 表单数据或二进制请求体发送图像，附带文件中的请求头、参数和查询参数，
并通过 `{json:…}`、`{regex:…}`、`{header:…}` 或 `{response}` 读取链接——也支持 ShareX 12 的 `$json:…$` 形式。
需要更多功能的文件（输入提示、随机值、XML 响应、并非图床的目标地址）会在导入时就被拒绝并说明原因，
而不是等到第一次上传时才失败。

在发送任何内容之前，Snipdeck 自己的窗口会列出图床及其地址，并说明这次上传意味着什么；
完成后，它会显示链接，并且已经替你复制好了。

---

## 📦 安装

### 下载

**Windows**——从 [**Releases**](https://github.com/ReviseTouch/snipdeck/releases)
页面获取 `snipdeck-windows-x64.exe`。单个便携可执行文件，无需安装程序。

**Linux (x64)**——从同一页面获取适合你发行版的软件包：

```bash
sudo apt install ./snipdeck_*_amd64.deb      # Debian, Ubuntu, Mint
sudo dnf install ./snipdeck-*.x86_64.rpm     # Fedora, RHEL, Rocky
sudo zypper install ./snipdeck-*.x86_64.rpm  # openSUSE
```

或者，在任何发行版上都无需 root 的 tarball：

```bash
tar xzf snipdeck-linux-x64.tar.gz
cd snipdeck-linux-x64
./install.sh          # into ~/.local; --uninstall reverses it
```

这些软件包可在任何带有 glibc 2.30 或更高版本的发行版上运行——Debian 11、Ubuntu 20.04、
RHEL 9 及之后的所有版本——并已在 Debian 12 和 13、Ubuntu 22.04 和 24.04、Fedora、Rocky 9、
openSUSE Leap 15.6 和 Tumbleweed 上完成安装和启动（`packaging/linux/test-install.sh`）。
在 GNOME 和 KDE Plasma 上，它们还会安装 Snipdeck 的桌面助手（见下文），Snipdeck 会在首次启动时
启用它；GNOME 会在下次登录时加载新安装的扩展。

### 从源码构建

要求：[Rust](https://rustup.rs/)（稳定版）。

```bash
git clone https://github.com/ReviseTouch/snipdeck.git
cd snipdeck
cargo build --release
```

**Windows**（10 或 11）：生成的二进制文件位于 `target\release\snipdeck.exe`。

**Linux**：构建需要 GTK、X11/Wayland、托盘、D-Bus 和 GL 的头文件。
在 Debian/Ubuntu 上：

```bash
sudo apt install libgtk-3-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxcb-randr0-dev libxrandr-dev libxkbcommon-dev \
  libwayland-dev libayatana-appindicator3-dev libfontconfig-dev libdbus-1-dev \
  libgl1-mesa-dev libegl1-mesa-dev
```

PipeWire 在构建时无需任何依赖：它会在运行时加载，用于 Wayland 截图。
若要像发行版那样安装你自己构建的版本，请将其与安装脚本放在一起：

```bash
packaging/linux/install-from-source.sh   # builds, stages afresh, installs
```

---

## 🌍 语言

Snipdeck 已完全本地化。首次启动时它会根据操作系统区域设置选择你的语言；
你可以随时从工具栏的 **`⋯` → 语言** 中覆盖该设置。

> 英语 · 西班牙语 · 简体中文 · 印地语 · 阿拉伯语 · 葡萄牙语 · 俄语 · 日语 ·
> 德语 · 法语 · 韩语 · 意大利语 · 土耳其语 · 印度尼西亚语 · 越南语 · 波兰语 · 乌克兰语 ·
> 荷兰语 · 波斯语 · 泰语 · 孟加拉语 · 乌尔都语 · 罗马尼亚语

整个应用——无论是 Slint 界面（`@tr`）还是 Rust 端的消息（`i18n::t`）——都从
位于 `lang/<code>/LC_MESSAGES/snipdeck.po` 的**每种语言一个 gettext `.po` 文件**读取，
并在构建时打包进二进制文件（运行时无 gettext 依赖）。

<details>
<summary><b>添加或更新翻译</b></summary>

1. 新字符串需用 `@tr("…")`（在 `.slint` 中）或 `i18n::t("…")`（在 Rust 中）包裹。
2. `lang/snipdeck.pot` 是列出每个英文字符串的模板。
3. 将新的 msgid 复制到每个 `lang/<code>/LC_MESSAGES/snipdeck.po` 中并填写其 `msgstr`——
   保持占位符（`{}`、`{e}`、`{url}`、`{path}`）和 `\r\n` 转义符完好无损。
4. 要添加新语言，请创建其 `.po` 文件，并将语言代码加入 `src/i18n.rs` 中的
   `LANGUAGES` 和 `po_source()`。
5. 运行 `tools/i18n-check.py`：它检查每条字符串都已翻译且占位符完整；CI 也会运行它。

</details>

---

## 🔒 隐私与安全

- **截图和 OCR 均在本地完成。** OCR 在 Windows 上使用 `Windows.Media.Ocr`，
  在 Linux 上使用 Tesseract（通过管道传入，不产生临时文件）；截图或建立索引时
  不会向任何地方发送任何内容。在 Wayland 上，屏幕通过 ScreenCast 门户读取：
  你只需授权一次，共享指示器仅在截图的那一刻显示。
- **OCR + 翻译会先征求同意。** 首次使用时，它会说明经 OCR 识别出的*文本*——绝不是图像——
  将发送到 Google 翻译，并等待你同意。
  文本放在请求正文中发送，而不是放在 URL 里。
- **上传每次都会先征求同意。** 在你于窗口中按下 **上传** 之前，任何内容都不会离开你的机器；
  该窗口会列出图床及其地址，并说明这意味着什么：任何拥有链接的人都能打开图像，上传到 catbox 的内容无法删除，
  普通 `http://` 服务器会以未加密方式发送。请勿将敏感内容上传到凭链接即可公开访问的图床。
  从导入的 `.sxcu` 文件中获得的 API 密钥保存在 `settings.json` 中，与 ShareX 的做法相同。
- **转交副本保持私密。** 传给邮件客户端或编辑器的图像会放入只有你能读取的文件夹
  （Linux 上为 `$XDG_RUNTIME_DIR`，Windows 上为 `%TEMP%\Snipdeck`），
  并在之后被清理。
- **日志不留痕迹。** 指针坐标和窗口标题仅在调试级别下
  记录。
- **开机自启动基于文件。** 在 Windows 上是 `shell:Startup` 中的普通快捷方式
  （而不是安全工具最常标记的 `HKCU\…\Run` 键），在 Linux 上是 XDG
  自启动条目。

---

## 🗺️ 路线图

- [x] Windows 10 / 11
- [x] **Linux / GNOME**（Wayland）——借助随附的 Shell 扩展：用 `Super`+`Alt`+拖动
      或热键截图，截图放置在截取处并保持置顶，
      OCR，取色器
- [x] **Linux / KDE Plasma**（Wayland）——借助随附的 KWin 脚本：截图放置在截取处并保持置顶；
      通过热键和工具栏截图，OCR，取色器
- [x] **Linux / X11**（Xfce、MATE、Cinnamon、Xorg 上的 GNOME……）——全部功能，
      截图由窗口管理器保持置顶
- [~] **Linux / wlroots**（sway、Hyprland、river）——通过热键和工具栏截图，
      OCR，取色器；截图既不会放置在截取处，也不会保持置顶（见下文）

- [ ] **macOS**——即将推出
- [ ] 应用内设置窗口
- [ ] 更多标注工具（模糊 / 像素化、带编号的步骤标记、移动/编辑形状）

GNOME 之外没有修饰键+拖动手势：它需要全局指针钩子，而 Linux 只能通过读取
`/dev/input` 来提供——这相当于一个系统级键盘记录器，Snipdeck 不会要求你启用。

### 在 Linux 上运行

在 Wayland 上，全局快捷键要经过桌面门户，而门户会拒绝来自没有应用 ID 的客户端的会话。
从应用程序菜单启动时，Snipdeck 拥有应用 ID（`com.revisetouch.Snipdeck`，即其桌面条目的名称）；
从终端启动时，它会自行注册同一个 ID，这需要 xdg-desktop-portal 1.19 或更高版本——
若门户版本较旧，请从菜单启动。

运行时依赖：`libayatana-appindicator`（托盘）、`xdg-utils`（分享），
以及 `tesseract` 和你需要的语言数据（OCR——可选；没有它时 OCR 会被跳过，
而不会报错）。

**Wayland 不允许的事。** Wayland 客户端无法自行定位窗口、查询窗口位置，
也无法要求窗口保持在其他窗口之上；这三件事都由合成器掌管。因此在纯 Wayland
会话中，截图会出现在合成器安排的位置，而不是截取区域的上方；它无法
悬浮在其他应用之上，并且会出现在窗口列表中。其他一切——区域截图、冻屏覆盖层、
全局快捷键、OCR、拖动、右键菜单——都能正常工作。

**在 KDE Plasma 上，一个 KWin 脚本起到同样的作用**（`packaging/kwin-script/`）：
它让截图保持在其他窗口之上，将它们放到截取时的位置，在所有桌面上显示，并让它们不出现在任务栏中。
无需重新登录即可加载。

在 wlroots 系合成器（sway、Hyprland、river）上，目前还没有任何机制能做到这一点：截图会出现在
合成器安排的位置，也不会保持置顶。可以在合成器自身的配置中添加窗口规则让它们浮动——
它们的标题为 `SnipdeckFloating …`。

**在 GNOME 上，一个 Shell 扩展弥补了这一缺口。** 客户端做不到的事，
运行在 GNOME Shell 内部的代码可以做到。因此，`install.sh` 在检测到
GNOME 会话时，还会安装 `snipdeck@revisetouch.com`（来自 `packaging/gnome-extension/`）——
这是一个单文件扩展，它监视 Snipdeck 的截图窗口，将它们放到截取时的位置，
保持在其他窗口之上，固定到所有工作区，并把它们从窗口切换器中移除。
位置信息通过窗口标题传递，而在这些无边框窗口上标题永远不可见。

该扩展还恢复了截图手势。在 GNOME 上 `Super`+拖动已用于移动窗口，
因此手势改为 **`Super`+`Alt`+拖动**，在 Snipdeck 运行期间有效：

| 手势（GNOME） | 结果 |
|---|---|
| `Super`+`Alt` + 拖动 | 屏幕上的悬浮截图 |
| `Super`+`Alt`+`Ctrl` + 拖动 | 悬浮截图**并**复制到剪贴板 |
| `Super`+`Alt`+`Shift` + 拖动 | 仅图库（无悬浮窗口） |
| `Super`+`Alt`+`Ctrl`+`Shift` + 拖动 | 图库**与**剪贴板，无悬浮窗口 |

“仅剪贴板”（在 Windows 上是 `Win`+`Alt`）在这里没有对应的手势；请使用它的
热键。`Esc` 可取消拖动；持续按住 `Super`+`Alt` 则可连续截取
多张截图。

安装后请注销并重新登录；在 Wayland 上 GNOME Shell 无法原地重新加载扩展，
因为它*本身*就是承载你会话的合成器。该扩展完全可选——没有它，
Snipdeck 的表现与任何其他 Wayland 客户端无异——而且除了 Snipdeck 自己的窗口之外，
它只接管 `Super`+`Alt`+拖动，并且仅在 Snipdeck 运行期间。

GNOME Wayland 上仍欠缺的一点：你拖动的截图由 Mutter 移动，而应用永远不会
得知它停在了哪里，因此其位置不会在重启后被记住。

---

## ⚙️ 配置

设置以 JSON 形式存储，并在首次运行时创建。目前尚无设置窗口——请直接编辑该
文件，然后重启 Snipdeck。

| 路径 | 内容 |
| --- | --- |
| `%APPDATA%\Snipdeck\settings.json` | 用户设置（见下） |
| `%LOCALAPPDATA%\Snipdeck\` | `snips.db`、`state.json`，以及图像 + 缩略图的 `cache\` |
| `Pictures\Snipdeck\` | *保存*的默认文件夹 |

<details>
<summary><b>部分 <code>settings.json</code> 键</b></summary>

| 键 | 默认值 | 含义 |
| --- | --- | --- |
| `language` | `"auto"` | 界面语言代码，或填 `"auto"` 以跟随操作系统 |
| `dark_theme` | `true` | 浅色或深色外壳 |
| `upload_service` | `"catbox"` | *上传*所用的图床：`"catbox"`（无需密钥）、`"imgur"` 或 `"custom:<name>"`；在 `⋯` → 上传到 中设置 |
| `upload_hosts` | `[]` | 从 ShareX `.sxcu` 文件导入的图床，沿用 ShareX 自身的字段名 |
| `imgur_client_id` | `""` | Imgur Client-ID，仅当 `upload_service` 为 `"imgur"` 时需要 |
| `translate_target` | `"en"` | 默认的 OCR 翻译目标语言代码 |
| `translate_favs` | `["en","es","fr","de","it","pt","ru","ar"]` | 翻译弹窗中可快速选取的语言 |
| `default_format` | `"png"` | *保存*的默认扩展名 |

**上传服务**——*catbox.moe*（默认）无需账户或密钥，但匿名上传
**事后无法由你删除**。*Imgur* 需要一个免费的 Client-ID（在 imgur.com → Settings →
Applications 注册一次即可）。两种情况下，图像均为不公开列出，但任何拥有
链接的人都能打开它，且未经加密。

</details>

---

## 🪶 资源占用

Snipdeck 是原生代码：Rust，配合 [Slint](https://slint.dev) 界面。它没有内嵌浏览器引擎（不像 Electron 应用），也不依赖 .NET 或 Java 运行时，因此只需一个文件即可分发，运行时也保持轻量。

| | 图库打开，空闲 | 在托盘中待命 |
| --- | --- | --- |
| 内存（PSS） | 68 MB | 67 MB |
| 其中自身独占（USS） | 57 MB | 57 MB |
| CPU | 0.6 % | 0.05 % |

| 下载 | 大小 |
| --- | --- |
| Windows, `snipdeck-windows-x64.exe` | 32 MB，完整应用 |
| Linux, `.deb` | 10 MB |

*测量方法：* 在 Linux x64（GNOME 51，Wayland）上运行 0.3.0 发布版构建，图库中有 10 张截图，静置 25 秒后测量；内存读取自 `/proc/<pid>/smaps_rollup`，CPU 取 40 秒平均值。PSS 将共享库（GTK、Mesa）按其使用者数量分摊计算；USS 是仅由 Snipdeck 占用的内存。测量日期为 2026-10-15；Windows 尚未测量。

## 🧩 架构

一个单线程的 Slint 事件循环驱动界面；操作系统钩子和阻塞性工作在各自的
线程上运行，并通过通道将结果交回，每个 tick 排空一次。

<details>
<summary><b>模块图</b></summary>

| 模块 | 职责 |
| --- | --- |
| `src/app/` | 应用状态、每 tick 循环、界面操作、图库模型 |
| `src/capture.rs`、`src/dxgi_capture.rs` | 显示器截图（DXGI 桌面复制，GDI 回退） |
| `src/mouse_trigger.rs`、`src/native_mouse_trigger.rs` | 用于预备和驱动截图的底层鼠标/键盘钩子 |
| `src/slint_overlay.rs`、`ui/overlay.slint` | 全屏选区覆盖层（实时或冻结） |
| `src/floating.rs`、`ui/floating.slint` | 悬浮截图窗口（拖动、缩放、裁剪、边框） |
| `src/annotate.rs`、`ui/annotate.slint` | 标注编辑器（tiny-skia 拼合、HSV 色轮、缩放） |
| `src/ocr.rs` | `Windows.Media.Ocr` 封装（运行于工作线程） |
| `src/translate.rs`、`src/translate_popup.rs` | OCR 翻译弹窗 |
| `src/share.rs`、`src/upload.rs` | 分享面板 / MAPI 邮件 / 编辑器 / 图床上传 |
| `src/sxcu.rs`、`src/upload_window.rs`、`ui/upload.slint` | ShareX 自定义上传器；上传的同意 / 进度 / 结果窗口 |
| `src/tray.rs`、`src/autostart.rs`、`src/single_instance.rs` | 托盘图标、开机自启动、单实例守卫 |
| `src/snip.rs`、`src/settings.rs`、`src/paths.rs` | 图库持久化（SQLite + FTS）、设置、路径 |
| `src/image_files.rs`、`src/atomic_file.rs` | 磁盘上的截图图像：无损 WebP、原子写入 |
| `src/i18n.rs`、`lang/` | 运行时 i18n + 打包的 gettext `.po` 目录（23 种语言） |

</details>

---

## ✨ 由 ReviseTouch 打造

Snipdeck 是 **[ReviseTouch](https://revisetouch.com)** 出品的一款免费开源应用——
我们开发开发者工具和 AI 产品。想了解我们还在打造什么，请访问
**[revisetouch.com](https://revisetouch.com)**。

---

## License

[MIT](LICENSE) © 2026 Hasan Salihoğlu

<div align="center">
<sub>由 🦀 Rust 和 <a href="https://slint.dev">Slint</a> 构建。</sub>
</div>
