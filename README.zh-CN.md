<div align="center">

[English](README.md) · [Türkçe](README.tr.md) · [Español](README.es.md) · **简体中文** · [العربية](README.ar.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · [日本語](README.ja.md) · [Français](README.fr.md) · [Português](README.pt.md)

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**一款快速、以键盘为核心的 Windows 屏幕截图 _工作台_。**

框选一块区域，让截图悬浮于一切窗口之上，对其进行标注、
建立 OCR 索引，并在一个高密度图库中检索数百张截图——
这一切都来自一个原生 Rust 应用，搭配软件渲染的 [Slint](https://slint.dev) 界面。

![platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows&logoColor=white)
![soon](https://img.shields.io/badge/macOS%20%2F%20Linux-on%20the%20way-lightgrey)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Slint-orange?logo=rust&logoColor=white)
![languages](https://img.shields.io/badge/languages-23-success)
![license](https://img.shields.io/badge/license-MIT-green)

</div>

---

## 📸 截图

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · translate.png
-->

<div align="center">

| 图库工作区 | 桌面上的悬浮截图 |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="图库" width="420" /> | <img src="docs/screenshots/floating.png" alt="悬浮截图" width="420" /> |
| **标注编辑器** | **OCR + 翻译** |
| <img src="docs/screenshots/annotate.png" alt="标注编辑器" width="420" /> | <img src="docs/screenshots/translate.png" alt="OCR 翻译" width="420" /> |

</div>

---

## ✨ 亮点

- 🖱️ **按住修饰键 + 拖动即可截图。** 按住 `Win` 并拖动框选一块区域；由修饰键决定
  结果如何处理——悬浮窗口、剪贴板或仅入图库（见下方矩阵）。
- 🧊 **冻屏优先截图**（`Win`+`Shift`+`Space`）。在你选择*之前*先将屏幕变暗并冻结，
  这样悬停提示及其他对点击敏感的界面都能被截下。
- 📌 **悬浮截图。** 截图会作为轻量、无边框的覆盖层固定在最上层，你可以
  拖动、缩放、固定、裁剪、旋转/翻转以及关闭它们。
- ✏️ **内置标注编辑器。** 画笔、荧光笔、形状、箭头和文字，配有完整的 HSV
  色轮、可调节的大小 + 不透明度、缩放以及撤销/重做（见详情）。
- 🔍 **内置 OCR。** 每张截图都会在后台用操作系统引擎
  （`Windows.Media.Ocr`）建立 OCR 索引——无需下载模型、无需联网，完全本地运行。
- 🌐 **OCR + 翻译。** 内联读取截图中的文字并译为另一种语言（30+ 种目标语言，
  可标星收藏常用项）。
- 🗂️ **强大的图库。** 对 OCR 文本、窗口元数据、显示器和路径进行全文检索；
  按状态 / 内容 / 大小 / 日期筛选；虚拟化网格在数千张截图下依然流畅。
- 📤 **分享与上传。** Windows 分享面板、新建邮件（Simple MAPI）、系统图像编辑器，
  或图床——上传**默认关闭，需通过明确的同意对话框选择启用**。
- 🌍 **23 种语言**，根据你的操作系统自动检测，并可从工具栏切换。
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
  或**上传**到图床
- **变换**——旋转（90° / 180°）、水平 / 垂直翻转
- **裁剪**（向内拖动手柄）和**调整大小**
- **从屏幕隐藏** / **删除**

---

## 📦 安装

### 下载

从 [**Releases**](https://github.com/hasantr/snipdeck/releases)
页面获取最新的 `snipdeck.exe`。它是单个便携可执行文件——无需安装程序。

### 从源码构建

要求：[Rust](https://rustup.rs/)（稳定版）和 Windows 10 或 11。

```powershell
git clone https://github.com/hasantr/snipdeck.git
cd snipdeck
cargo build --release
```

生成的二进制文件位于 `target\release\snipdeck.exe`。

---

## 🌍 语言

Snipdeck 已完全本地化。首次启动时它会根据操作系统区域设置选择你的语言；
你可以随时从工具栏的**地球菜单**中覆盖该设置。

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

</details>

---

## 🔒 隐私与安全

- **截图和 OCR 完全本地运行。** OCR 使用内置的 `Windows.Media.Ocr` 引擎；截图或建立索引时
  不会向任何地方发送任何内容。
- **开机自启动不涉及注册表。** Snipdeck 会在你的 `shell:Startup` 文件夹中放置一个普通快捷方式，
  而不是写入 `HKCU\…\Run` 值——这是一种普通的、用户可见的机制，从而
  避开杀毒软件 / EDR 工具最常标记的那个自启动注册表项。
- **上传默认关闭。** 未经明确说明目的地的同意对话框确认，任何内容都不会离开你的机器。
  请勿将敏感内容上传到凭链接即可公开访问的图床。
- **OCR + 翻译会使用网络。** 仅将 OCR 识别出的*文本*发送到 Google 的公共
  `translate_a` 端点进行翻译；它不会被用于其他任何用途。

---

## 🗺️ 路线图

- [x] Windows 10 / 11
- [ ] **macOS**——即将推出
- [ ] **Linux**——即将推出
- [ ] 应用内设置窗口
- [ ] 更多标注工具（模糊 / 像素化、带编号的步骤标记、移动/编辑形状）

> Snipdeck 目前仅支持 Windows，因为截图、OCR、托盘和分享全都使用
> Windows 专有 API。界面（[Slint](https://slint.dev)）和大部分核心已经
> 跨平台，因此 **macOS 和 Linux 版本即将推出。**

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
| `upload_service` | `"catbox"` | *上传*所用的图床：`"catbox"`（无需密钥）或 `"imgur"` |
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
| `src/tray.rs`、`src/autostart.rs`、`src/single_instance.rs` | 托盘图标、开机自启动、单实例守卫 |
| `src/snip.rs`、`src/settings.rs`、`src/paths.rs` | 图库持久化（SQLite + FTS）、设置、路径 |
| `src/i18n.rs`、`lang/` | 运行时 i18n + 打包的 gettext `.po` 目录（23 种语言） |

</details>

---

## License

[MIT](LICENSE) © 2026 Hasan Salihoğlu

<div align="center">
<sub>由 🦀 Rust 和 <a href="https://slint.dev">Slint</a> 构建。</sub>
</div>
