<div align="center">

[English](README.md) · [Türkçe](README.tr.md) · [Español](README.es.md) · [简体中文](README.zh-CN.md) · [العربية](README.ar.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · **日本語** · [Français](README.fr.md) · [Português](README.pt.md)

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**高速でキーボード中心に操作できる、Windows 向け画面キャプチャ _デッキ_。**

範囲をキャプチャして、スニップを常に最前面のフローティング表示にしたまま、書き込みを加え、
OCR でインデックス化し、ひとつの密度の高いギャラリーから数百枚のスクリーンショットを検索できます ——
そのすべてを、ソフトウェアレンダリングの [Slint](https://slint.dev) UI を備えたネイティブ Rust アプリで実現します。

![platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows&logoColor=white)
![soon](https://img.shields.io/badge/macOS%20%2F%20Linux-on%20the%20way-lightgrey)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Slint-orange?logo=rust&logoColor=white)
![languages](https://img.shields.io/badge/languages-23-success)
![license](https://img.shields.io/badge/license-MIT-green)

</div>

---

## 📸 スクリーンショット

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · translate.png
-->

<div align="center">

| ギャラリーワークスペース | デスクトップ上のフローティングスニップ |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="ギャラリー" width="420" /> | <img src="docs/screenshots/floating.png" alt="浮遊スニップ" width="420" /> |
| **注釈エディター** | **OCR + 翻訳** |
| <img src="docs/screenshots/annotate.png" alt="注釈エディター" width="420" /> | <img src="docs/screenshots/translate.png" alt="OCR 翻訳" width="420" /> |

</div>

---

## ✨ 主な特徴

- 🖱️ **修飾キー + ドラッグでキャプチャ。** `Win` を押しながら範囲をドラッグします。修飾キーによって、
  結果をどう扱うかが決まります —— フローティングウィンドウ、クリップボード、ギャラリーのみ（下記のマトリックス）。
- 🧊 **フリーズ優先キャプチャ**（`Win`+`Shift`+`Space`）。選択する*前に*画面を暗くしてフリーズするため、
  ホバー時のツールチップやその他のクリックに反応する UI もそのままキャプチャできます。
- 📌 **フローティングスニップ。** キャプチャは軽量でボーダーレスのオーバーレイとして最前面にピン留めされ、
  ドラッグ、リサイズ、ピン留め、切り抜き、回転 / 反転、そして消去ができます。
- ✏️ **内蔵の注釈エディター。** ペン、ハイライター、図形、矢印、テキストを、フルの HSV
  カラーホイール、サイズ + 不透明度の調整、ズーム、元に戻す / やり直しとともに利用できます（詳細）。
- 🔍 **内蔵 OCR。** すべてのスニップは、OS のエンジン（`Windows.Media.Ocr`）によってバックグラウンドで
  OCR インデックス化されます —— モデルのダウンロード不要、ネットワーク不要、完全にローカルです。
- 🌐 **OCR + 翻訳。** スニップのテキストをその場で別の言語に翻訳して読めます（30 以上の対象言語、
  スターでお気に入り登録も可能）。
- 🗂️ **強力なギャラリー。** OCR テキスト、ウィンドウのメタデータ、モニター、パスを横断する全文検索。
  ステータス / コンテンツ / サイズ / 日付でフィルタリングでき、数千枚のスニップでもスムーズに動作する仮想化グリッドを備えます。
- 📤 **共有 & アップロード。** Windows の共有シート、新規メール（Simple MAPI）、システムの画像エディター、
  または画像ホスト —— アップロードは**明示的な同意ダイアログによるオプトイン**です。
- 🌍 **23 の言語**。OS から自動検出され、ツールバーから切り替えられます。
- 🌗 **ライト / ダークテーマ**、システムトレイのホーム、そして**レジストリを使わない**ログイン時起動。

---

## ⌨️ キャプチャショートカット

`Win` を押しながら、**左**マウスボタンで範囲をドラッグします。修飾キーを追加すると、
スニップの扱い方を変えられます。

| ショートカット | 結果 |
| --- | --- |
| `Win` + ドラッグ | 画面上にフローティングスニップ |
| `Win`+`Ctrl` + ドラッグ | フローティングスニップ**かつ**クリップボードにコピー |
| `Win`+`Alt` + ドラッグ | クリップボードのみ（ギャラリーには追加しない） |
| `Win`+`Shift` + ドラッグ | ギャラリーのみ（フローティングウィンドウなし） |
| `Win`+`Ctrl`+`Shift` + ドラッグ | ギャラリー**かつ**クリップボード、フローティングウィンドウなし |
| `Win`+`Shift`+`Space` の後にドラッグ | **フリーズ優先** —— 画面をフリーズして、静止画像の上で選択 |

トレイアイコンを右クリック → **新規スニップ** を選ぶと、`Win` を押さずに
一度きりのドラッグキャプチャを準備することもできます。

### グローバルホットキー

`Win` + ドラッグに加えて、Snipdeck は設定可能な 5 つのグローバルホットキーを登録します。

| ホットキー | 動作 |
| --- | --- |
| `Win`+`Alt`+`H` | 画面上のすべてのスニップを表示 / 非表示 |
| `Ctrl`+`Shift`+`A` | 新規スニップ |
| `Ctrl`+`Shift`+`C` | 新規スニップ**かつ**クリップボードにコピー |
| `Ctrl`+`Shift`+`X` | クリップボードのみ |
| `Ctrl`+`Shift`+`G` | ギャラリーを開く |

`settings.json` の `hotkey_*` キーで変更できます。

---

## ✏️ 注釈エディター

任意のスニップを右クリック → **注釈** を選ぶと、スニップの最前面に表示される
Telegram 風の集中できるエディターが開きます。

- **ツール** —— フリーハンドのペン、ハイライター、四角形、楕円、線、矢印、テキスト。
- **明るさスライダー付きのフル HSV カラーホイール** —— 固定パレットではなく、*任意の*色を選べます。
- **サイズ & 不透明度のスライダー**。画像上で点滅するサイズプレビュー付きで、ツールごとに記憶されます。
- **ズームイン / アウト / フィット** で精密な作業が可能。ズームインすると、キャンバスはパンします。
- **元に戻す / やり直し / クリア**、さらに `Esc`、`Ctrl`+`Z`、`Ctrl`+`Y`。
- **保存**時には、ストロークがフル解像度で画像に統合されます（くっきりとした、原寸サイズ）。

ライブプレビューと保存結果は同じコードから生成されるため、**見たままが得られます**。

---

## 🖱️ 右クリック操作

フローティングスニップ**または**ギャラリーのサムネイルを右クリックします。

- 画像をクリップボードに**コピー**
- **OCR + クリップボード** —— OCR を実行して認識されたテキストをコピー
- **OCR + 翻訳** —— 翻訳ポップアップを開く
- **注釈** —— 上記のエディターを開く
- **共有** —— Windows の共有シート、**メール**（MAPI 添付）、**エディター**（ペイント）で開く、
  または画像ホストへ**アップロード**
- **変換** —— 回転（90° / 180°）、水平 / 垂直反転
- **切り抜き**（ハンドルを内側へドラッグ）と**リサイズ**
- **画面から隠す** / **削除**

---

## 📦 インストール

### ダウンロード

最新の `snipdeck.exe` を [**Releases**](https://github.com/hasantr/snipdeck/releases)
ページから入手してください。インストーラー不要の、単一のポータブル実行ファイルです。

### ソースからビルド

要件: [Rust](https://rustup.rs/)（stable）と Windows 10 または 11。

```powershell
git clone https://github.com/hasantr/snipdeck.git
cd snipdeck
cargo build --release
```

バイナリは `target\release\snipdeck.exe` に生成されます。

---

## 🌍 言語

Snipdeck は完全にローカライズされています。初回起動時に OS のロケールから言語を選び、
ツールバーの**地球儀メニュー**からいつでも上書きできます。

> 英語 · スペイン語 · 簡体字中国語 · ヒンディー語 · アラビア語 · ポルトガル語 · ロシア語 · 日本語 ·
> ドイツ語 · フランス語 · 韓国語 · イタリア語 · トルコ語 · インドネシア語 · ベトナム語 · ポーランド語 · ウクライナ語 ·
> オランダ語 · ペルシア語 · タイ語 · ベンガル語 · ウルドゥー語 · ルーマニア語

アプリ全体 —— Slint UI（`@tr`）と Rust 側のメッセージ（`i18n::t`）の両方 —— は、
`lang/<code>/LC_MESSAGES/snipdeck.po` にある**言語ごとにひとつの gettext `.po`** から読み込まれ、
ビルド時にバイナリへ組み込まれます（実行時の gettext 依存はありません）。

<details>
<summary><b>翻訳の追加・更新</b></summary>

1. 新しい文字列は `@tr("…")`（`.slint` 内）または `i18n::t("…")`（Rust 内）でラップされます。
2. `lang/snipdeck.pot` は、すべての英語文字列を列挙したテンプレートです。
3. 新しい msgid を各 `lang/<code>/LC_MESSAGES/snipdeck.po` にコピーし、その `msgstr` を埋めます ——
   プレースホルダー（`{}`、`{e}`、`{url}`、`{path}`）と `\r\n` エスケープはそのまま保ちます。
4. 新しい言語を追加するには、その `.po` を作成し、`src/i18n.rs` の `LANGUAGES` と `po_source()`
   にコードを追加します。

</details>

---

## 🔒 プライバシーとセキュリティ

- **キャプチャと OCR は完全にローカルです。** OCR は内蔵の `Windows.Media.Ocr` エンジンを使用します。
  キャプチャやインデックス化のために、どこかへ何かが送信されることはありません。
- **ログイン時起動はレジストリを使いません。** Snipdeck は `HKCU\…\Run` の値を書き込む代わりに、
  通常のショートカットを `shell:Startup` フォルダーに配置します —— これは一般的でユーザーに見える仕組みであり、
  アンチウイルス / EDR ツールが最も頻繁に検知する自動起動レジストリキーを避けられます。
- **アップロードはオプトインです。** 宛先を明示する同意ダイアログなしには、何もマシンの外に出ません。
  リンクを知れば誰でも見られるホストへ、機密性の高いコンテンツをアップロードしないでください。
- **OCR + 翻訳はネットワークを使用します。** OCR された*テキスト*のみが、翻訳のために Google の公開
  `translate_a` エンドポイントへ送信されます。それ以外の用途には使われません。

---

## 🗺️ ロードマップ

- [x] Windows 10 / 11
- [ ] **macOS** —— 開発中
- [ ] **Linux** —— 開発中
- [ ] アプリ内設定ウィンドウ
- [ ] さらなる注釈ツール（ぼかし / モザイク、番号付きステップバッジ、図形の移動 / 編集）

> Snipdeck が現在 Windows 専用なのは、キャプチャ、OCR、トレイ、共有のすべてが
> Windows 固有の API を使っているためです。UI（[Slint](https://slint.dev)）とコアの大部分は
> すでにクロスプラットフォームなので、**macOS と Linux のビルドは開発中です。**

---

## ⚙️ 設定

設定は JSON として保存され、初回実行時に作成されます。設定ウィンドウはまだないため、
ファイルを直接編集してから、Snipdeck を再起動してください。

| パス | 内容 |
| --- | --- |
| `%APPDATA%\Snipdeck\settings.json` | ユーザー設定（下記） |
| `%LOCALAPPDATA%\Snipdeck\` | `snips.db`、`state.json`、および画像 + サムネイルの `cache\` |
| `Pictures\Snipdeck\` | *保存* のデフォルトフォルダー |

<details>
<summary><b>主な <code>settings.json</code> のキー</b></summary>

| キー | デフォルト | 意味 |
| --- | --- | --- |
| `language` | `"auto"` | UI の言語コード、または OS に従う場合は `"auto"` |
| `dark_theme` | `true` | ライトまたはダークのシェル |
| `upload_service` | `"catbox"` | *アップロード* 用の画像ホスト: `"catbox"`（キー不要）または `"imgur"` |
| `imgur_client_id` | `""` | Imgur の Client-ID。`upload_service` が `"imgur"` の場合のみ必要 |
| `translate_target` | `"en"` | OCR 翻訳のデフォルト対象言語コード |
| `translate_favs` | `["en","es","fr","de","it","pt","ru","ar"]` | 翻訳ポップアップでのクイック選択言語 |
| `default_format` | `"png"` | *保存* のデフォルト拡張子 |

**アップロードサービス** —— *catbox.moe*（デフォルト）はアカウントもキーも不要ですが、匿名アップロードは
**後から自分で削除できません**。*Imgur* は無料の Client-ID が必要です（imgur.com → Settings → Applications
で一度登録）。いずれの場合も画像は未掲載（unlisted）ですが、リンクを知っていれば誰でも開くことができ、
暗号化はされていません。

</details>

---

## 🧩 アーキテクチャ

シングルスレッドの Slint イベントループが UI を駆動します。OS フックやブロッキング処理は
それぞれ専用のスレッドで実行され、結果はチャネル経由で返され、ティックごとに 1 回排出されます。

<details>
<summary><b>モジュールマップ</b></summary>

| モジュール | 責務 |
| --- | --- |
| `src/app/` | アプリの状態、ティックごとのループ、UI アクション、ギャラリーモデル |
| `src/capture.rs`, `src/dxgi_capture.rs` | モニターキャプチャ（DXGI Desktop Duplication、GDI フォールバック） |
| `src/mouse_trigger.rs`, `src/native_mouse_trigger.rs` | スニップを準備・駆動する低レベルのマウス / キーボードフック |
| `src/slint_overlay.rs`, `ui/overlay.slint` | 全画面の選択オーバーレイ（ライブまたはフリーズ） |
| `src/floating.rs`, `ui/floating.slint` | フローティングスニップウィンドウ（ドラッグ、リサイズ、切り抜き、ボーダー） |
| `src/annotate.rs`, `ui/annotate.slint` | 注釈エディター（tiny-skia 統合、HSV ホイール、ズーム） |
| `src/ocr.rs` | `Windows.Media.Ocr` ラッパー（ワーカースレッド上） |
| `src/translate.rs`, `src/translate_popup.rs` | OCR 翻訳ポップアップ |
| `src/share.rs`, `src/upload.rs` | 共有シート / MAPI メール / エディター / 画像ホストアップロード |
| `src/tray.rs`, `src/autostart.rs`, `src/single_instance.rs` | トレイアイコン、ログイン時起動、単一インスタンスガード |
| `src/snip.rs`, `src/settings.rs`, `src/paths.rs` | ギャラリーの永続化（SQLite + FTS）、設定、パス |
| `src/i18n.rs`, `lang/` | 実行時 i18n + 組み込みの gettext `.po` カタログ（23 言語） |

</details>

---

## ライセンス

[MIT](LICENSE) © 2026 Hasan Salihoğlu

<div align="center">
<sub>🦀 Rust と <a href="https://slint.dev">Slint</a> で構築されています。</sub>
</div>
