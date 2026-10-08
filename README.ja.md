<div align="center">

[English](README.md) · [Türkçe](README.tr.md) · [Español](README.es.md) · [简体中文](README.zh-CN.md) · [العربية](README.ar.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · **日本語** · [Français](README.fr.md) · [Português](README.pt.md)

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**高速でキーボード中心に操作できる、Windows 向け画面キャプチャ _デッキ_。**

<sub><a href="https://revisetouch.com"><b>ReviseTouch</b></a> による無料のオープンソースアプリです。</sub>

範囲をキャプチャして、スニップを常に最前面のフローティング表示にしたまま、書き込みを加え、
OCR でインデックス化し、ひとつの密度の高いギャラリーから数百枚のスクリーンショットを検索できます ——
そのすべてを、ソフトウェアレンダリングの [Slint](https://slint.dev) UI を備えたネイティブ Rust アプリで実現します。

![platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows&logoColor=white)
![linux](https://img.shields.io/badge/Linux-GNOME%20full%20%2F%20others%20partial-yellow) ![soon](https://img.shields.io/badge/macOS-on%20the%20way-lightgrey)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Slint-orange?logo=rust&logoColor=white)
![languages](https://img.shields.io/badge/languages-23-success)
![license](https://img.shields.io/badge/license-MIT-green)

</div>

---

## 📸 スクリーンショット

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · collage.png
-->

<div align="center">

| ギャラリーワークスペース | デスクトップ上のフローティングスニップ |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="ギャラリー" width="420" /> | <img src="docs/screenshots/floating.png" alt="フローティングスニップ" width="420" /> |
| **注釈エディター** | **コラージュエディター** |
| <img src="docs/screenshots/annotate.png" alt="注釈エディター" width="420" /> | <img src="docs/screenshots/collage.png" alt="コラージュエディター" width="420" /> |

</div>

---

## ✨ 主な特徴

- 🖱️ **修飾キー + ドラッグでキャプチャ。** `Win` を押しながら範囲をドラッグします。修飾キーによって、
  結果をどう扱うかが決まります —— フローティングウィンドウ、クリップボード、ギャラリーのみ（[下記のマトリックス](#️-キャプチャショートカット)）。
- 🧊 **フリーズ優先キャプチャ**（`Win`+`Shift`+`Space`）。選択する*前に*画面を暗くしてフリーズするため、
  ホバー時のツールチップやその他のクリックに反応する UI もそのままキャプチャできます。
- 📌 **フローティングスニップ。** キャプチャは軽量でボーダーレスのオーバーレイとして最前面にピン留めされ、
  ドラッグ、リサイズ、ピン留め、切り抜き、回転 / 反転、そして消去ができます。
- ✏️ **内蔵の注釈エディター。** ペン、ハイライター、図形、矢印、テキストを、フルの HSV
  カラーホイール、サイズ + 不透明度の調整、ズーム、元に戻す / やり直しとともに利用できます（[詳細](#️-注釈エディター)）。
- 🧩 **コラージュ。** 複数のスニップを 1 枚の画像にレイアウトします —— グリッド、両端揃えの行、またはメイソンリー。
  間隔、角丸、背景も設定でき、新しいスニップとしてギャラリーに保存されます。
- 🔍 **内蔵 OCR。** すべてのスニップは、OS のエンジン（`Windows.Media.Ocr`）によってバックグラウンドで
  OCR インデックス化されます —— モデルのダウンロード不要、ネットワーク不要、完全にローカルです。
- 🌐 **OCR＋翻訳。** スニップのテキストをその場で別の言語に翻訳して読めます（30 以上の対象言語、
  スターでお気に入り登録も可能）。
- 🗂️ **強力なギャラリー。** OCR テキスト、ウィンドウのメタデータ、モニター、パスを横断する全文検索。
  ステータス / コンテンツ / サイズ / 日付でフィルタリングでき、数千枚のスニップでもスムーズに動作する仮想化グリッドを備えます
  （[ツアー](#️-ギャラリー)）。
- 📤 **共有 & アップロード。** Windows の共有シート、新規メール（Simple MAPI）、システムの画像エディター、
  または画像ホスト —— catbox.moe、Imgur、あるいは ShareX の `.sxcu` ファイルから追加した**自分のサーバー**
  （[詳細](#-自分の画像ホスト)）。アップロードは毎回**事前に確認します**。
- 🌍 **23 の言語**。OS から自動検出され、ツールバーの `⋯` メニューから切り替えられます。
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
| `Ctrl`+`Alt`+`Shift`+`A` | 新規スニップ |
| `Ctrl`+`Alt`+`Shift`+`C` | 新規スニップ**かつ**クリップボードにコピー |
| `Ctrl`+`Alt`+`Shift`+`X` | クリップボードのみ |
| `Ctrl`+`Alt`+`Shift`+`G` | ギャラリーを開く |

`settings.json` の `hotkey_*` キーで変更できます。Wayland では
グローバルショートカットはデスクトップ側が管理します。これらは初回起動時に提案されるだけで、
その後はデスクトップのキーボード設定から変更します。

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
- **OCR＋クリップボード** —— OCR を実行して認識されたテキストをコピー
- **OCR＋翻訳** —— 翻訳ポップアップを開く
- **注釈** —— 上記のエディターを開く
- **共有** —— Windows の共有シート、**メール**（MAPI 添付）、**エディター**（ペイント）で開く、
  または画像ホストへ**アップロード**（[どのホストか](#-自分の画像ホスト)）
- **変形** —— 回転（90° / 180°）、水平 / 垂直反転
- **切り抜き**（ハンドルを内側へドラッグ）と**サイズ変更**
- **画面から非表示** / **削除**

---

## 🗂️ ギャラリー

ツールバーは 1 行だけ。それ以外はすべてワンクリックで届きます。

- **新規** でスニップを撮影します。その矢印には **スニップ＋クリップ** と **クリップのみ** があり、それぞれにホットキーが付いています。
- その隣に **コラージュ** と **色を取得** が並び、行の残りは検索ボックスが占めます ——
  `Ctrl`+`F` でフォーカス、`Esc` でクリア。
- **フィルター** は、ステータス、コンテンツ、サイズ、日付をひとつのパネルで開きます。有効なフィルターはツールバーの下に
  チップとして表示され、個別に外すことも、**フィルターをクリア** でまとめて外すこともできます。
- **`⋯`** には、めったに変えないものがまとまっています: 「タスクバーにフローティングスニップを表示」、「ダークテーマ」、
  「ログイン時に起動」、「言語」、ギャラリーの消去、そして「Snipdeck について」。
- ステータスバーには表示中の件数が示され、**並替** と **S / M / L** のカードサイズがあります。

カードにホバーするとクイックアクションが表示され、右クリックすると[その他すべての操作](#️-右クリック操作)が使えます。

---

## 📤 自分の画像ホスト

**`⋯` → アップロード先** で、**アップロード** がスニップを送る先を選びます: catbox.moe（デフォルト、アカウント不要）、
Imgur、または自分のホストです。追加するには **.sxcu ファイルから追加…** を選び、ShareX のカスタムアップローダーファイルを
指定します。Zipline、XBackBone、Chevereto をはじめ、セルフホストの画像サーバーの多くがすでに提供している形式なので、
手で書くものは何もありません。

Snipdeck は画像を multipart フォームデータまたはバイナリボディとして、ファイルに記述されたヘッダー、引数、
クエリパラメーターとともに送信し、リンクを `{json:…}`、`{regex:…}`、`{header:…}`、`{response}` で読み取ります
—— ShareX 12 の `$json:…$` 形式にも対応します。それ以上を必要とするファイル（入力プロンプト、ランダム値、
XML の応答、画像ホストではない送信先）は、最初のアップロードで失敗するのではなく、インポートの時点で理由とともに
拒否されます。

何かを送信する前に、Snipdeck 自身のウィンドウがホストとそのアドレスを示し、アップロードが何を意味するかを説明します。
完了すると、リンクを表示します。リンクはすでにコピーされています。

---

## 📦 インストール

### ダウンロード

**Windows** —— [**Releases**](https://github.com/hasantr/snipdeck/releases) ページから
`snipdeck-windows-x64.exe` を入手してください。インストーラー不要の、単一のポータブル実行ファイルです。

**Linux (x64)** —— 同じページから `snipdeck-linux-x64.tar.gz` を入手します。

```bash
tar xzf snipdeck-linux-x64.tar.gz
cd snipdeck-linux-x64
./install.sh          # into ~/.local, no root; --uninstall reverses it
```

その後、アプリケーションメニューから起動してください。これは好みの問題ではありません。
Wayland ではデスクトップポータルが、アプリの起動に使われた `.desktop` エントリーから
アプリケーション ID を導き出すため、それがないとグローバルショートカットを一切登録してくれません。

### ソースからビルド

要件: [Rust](https://rustup.rs/)（stable）。

```bash
git clone https://github.com/hasantr/snipdeck.git
cd snipdeck
cargo build --release
```

**Windows**（10 または 11）: バイナリは `target\release\snipdeck.exe` に生成されます。

**Linux**: ビルドには GTK、X11/Wayland、トレイ、D-Bus、GL のヘッダーが必要です。
Debian/Ubuntu では:

```bash
sudo apt install libgtk-3-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxcb-randr0-dev libxrandr-dev libxkbcommon-dev \
  libwayland-dev libayatana-appindicator3-dev libfontconfig-dev libdbus-1-dev \
  libgl1-mesa-dev libegl1-mesa-dev
```

PipeWire はビルド時には何も必要ありません。Wayland でのキャプチャ用に実行時に読み込まれます。
リリース版と同じ方法で自分のビルドをインストールするには、インストーラーと同じ場所に配置します。

```bash
mkdir -p dist/snipdeck && cp target/release/snipdeck packaging/linux/snipdeck.desktop \
  packaging/linux/install.sh dist/snipdeck/
cp -r packaging/gnome-extension/snipdeck@revisetouch.com dist/snipdeck/
cp assets/preview_256.png dist/snipdeck/snipdeck.png
dist/snipdeck/install.sh
```

---

## 🌍 言語

Snipdeck は完全にローカライズされています。初回起動時に OS のロケールから言語を選び、
ツールバーの **`⋯` → 言語** からいつでも上書きできます。

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

- **キャプチャと OCR はローカルで完結します。** OCR は Windows では `Windows.Media.Ocr`、
  Linux では Tesseract を使用します（パイプ経由で渡し、一時ファイルは作りません）。キャプチャや
  インデックス化のために、どこかへ何かが送信されることはありません。Wayland では画面は
  ScreenCast ポータル経由で読み取られます。許可は一度だけで、共有インジケーターは
  スニップを撮影する瞬間にしか表示されません。
- **OCR＋翻訳は事前に確認します。** 初回は、OCR で読み取った*テキスト*（画像は決して送られません）が
  Google 翻訳に送られることを説明し、あなたの同意を待ちます。
  テキストは URL ではなく、リクエストボディで送信されます。
- **アップロードは毎回、事前に確認します。** ホストとそのアドレスを示し、それが何を意味するか
  （リンクを知っていれば誰でも画像を開ける、catbox へのアップロードは削除できない、平文の `http://` サーバーには
  暗号化されずに送られる）を説明するウィンドウで **アップロード** を押すまで、何もマシンの外に出ません。
  リンクを知れば誰でも見られるホストへ、機密性の高いコンテンツをアップロードしないでください。
  インポートした `.sxcu` ファイルの API キーは、ShareX と同じく `settings.json` に保存されます。
- **受け渡し用のコピーは非公開のままです。** メールクライアントやエディターに渡す画像は、
  あなただけが読めるフォルダー（Linux では `$XDG_RUNTIME_DIR`、
  Windows では `%TEMP%\Snipdeck`）に置かれ、使用後に削除されます。
- **ログに痕跡は残りません。** ポインター座標とウィンドウタイトルは、
  デバッグレベルでのみ記録されます。
- **ログイン時起動はファイルベースです。** Windows では `shell:Startup` 内の通常のショートカット
  （セキュリティツールが最も頻繁に検知する `HKCU\…\Run` キーではありません）、Linux では XDG
  自動起動エントリーを使います。

---

## 🗺️ ロードマップ

- [x] Windows 10 / 11
- [x] **Linux / GNOME**（Wayland）—— 同梱のシェル拡張機能により: `Super`+`Alt`+ドラッグ
      またはホットキーでのキャプチャ、キャプチャした位置へのスニップ配置と最前面表示、
      OCR、カラーピッカー
- [~] **Linux / その他のデスクトップ** —— ホットキーとツールバーからのキャプチャ、
      OCR、カラーピッカー（デスクトップポータル経由）。GNOME 以外には
      修飾キー＋ドラッグのジェスチャーはありません。それにはグローバルなポインターフックが必要で、
      Linux では `/dev/input` を読むことでしか得られません —— つまりシステム全体のキーロガーであり、
      Snipdeck がそれを有効にするよう求めることはありません。Wayland では、拡張機能なしだと
      配置と常に最前面表示が欠けます（下記参照）
- [ ] **macOS** —— 開発中
- [ ] アプリ内設定ウィンドウ
- [ ] さらなる注釈ツール（ぼかし / モザイク、番号付きステップバッジ、図形の移動 / 編集）

### Linux での実行

インストールしたら、ターミナルからではなく**アプリケーションメニューから起動してください**。
グローバルショートカットはデスクトップポータルを経由しますが、ポータルはアプリケーション ID を
持たないクライアントからのセッションを拒否します。サンドボックス化されていないアプリが ID を得られるのは、
デスクトップが `.desktop` エントリーから起動したときだけです —— `packaging/linux/snipdeck.desktop` を参照してください。

実行時の依存関係: `libayatana-appindicator`（トレイ）、`xdg-utils`（共有）、
そして `tesseract` と必要な言語データ（OCR —— 任意。ない場合、OCR は
失敗せずにスキップされます）。

**Wayland で許されないこと。** Wayland クライアントは、自分のウィンドウを配置することも、
その位置を問い合わせることも、他より前面に保つよう要求することもできません。これら 3 つはすべて
コンポジターの管轄です。そのため素の Wayland セッションでは、スニップはキャプチャした範囲の上ではなく
コンポジターが決めた場所に開き、他のアプリケーションより前面に浮かぶことができず、
ウィンドウ一覧にも表示されます。それ以外のすべて —— 範囲キャプチャ、フリーズオーバーレイ、
グローバルショートカット、OCR、ドラッグ、コンテキストメニュー —— は動作します。

KDE と wlroots 系コンポジター（sway、Hyprland、river、COSMIC）は、まさにこの 3 つを可能にする
`wlr-layer-shell` を提供していますが、Snipdeck はまだ使っていません。
GNOME/Mutter はその実装を見送っています。

**GNOME では、シェル拡張機能がこの溝を埋めます。** クライアントにできないことも、
GNOME Shell の内部で動くコードならできます。そのため `install.sh` は GNOME セッションを検出すると、
`snipdeck@revisetouch.com`（`packaging/gnome-extension/` から）もインストールします ——
Snipdeck のスニップウィンドウを監視し、キャプチャした位置に配置し、他のウィンドウより前面に保ち、
すべてのワークスペースに固定し、ウィンドウスイッチャーから外す、単一のファイルです。
配置情報はウィンドウタイトルで受け渡されますが、ボーダーレスのこれらのウィンドウでタイトルが見えることはありません。

拡張機能はキャプチャのジェスチャーも復活させます。GNOME では `Super`+ドラッグがすでに
ウィンドウの移動に使われているため、ジェスチャーは **`Super`+`Alt`+ドラッグ** となり、
Snipdeck の実行中に有効です。

| ジェスチャー（GNOME） | 結果 |
|---|---|
| `Super`+`Alt` + ドラッグ | 画面上にフローティングスニップ |
| `Super`+`Alt`+`Ctrl` + ドラッグ | フローティングスニップ**かつ**クリップボードにコピー |
| `Super`+`Alt`+`Shift` + ドラッグ | ギャラリーのみ（フローティングウィンドウなし） |
| `Super`+`Alt`+`Ctrl`+`Shift` + ドラッグ | ギャラリー**かつ**クリップボード、フローティングウィンドウなし |

「クリップボードのみ」（Windows では `Win`+`Alt`）にはここではジェスチャーがないため、
そのホットキーを使ってください。`Esc` でドラッグを取り消せます。また `Super`+`Alt` を押したままにすると、
続けて複数のスニップを撮影できます。

インストール後はいったんログアウトして再ログインしてください。Wayland では GNOME Shell 自体が
セッションを保持するコンポジター*そのもの*であるため、拡張機能をその場で再読み込みできません。
拡張機能は完全に任意です —— なくても Snipdeck は他の Wayland クライアントと同じように動作します ——
また、Snipdeck 自身のウィンドウ以外で触れるのは `Super`+`Alt`+ドラッグだけで、
それも Snipdeck の実行中に限られます。

GNOME Wayland でまだ欠けている点: ドラッグしたスニップは Mutter が移動させ、アプリには
どこに置かれたかが伝えられないため、その位置は再起動をまたいで記憶されません。

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
| `upload_service` | `"catbox"` | *アップロード* 用の画像ホスト: `"catbox"`（キー不要）、`"imgur"`、または `"custom:<name>"`。`⋯` → アップロード先 で設定 |
| `upload_hosts` | `[]` | ShareX の `.sxcu` ファイルからインポートしたホスト。フィールド名は ShareX 自身のもの |
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
| `src/sxcu.rs`, `src/upload_window.rs`, `ui/upload.slint` | ShareX カスタムアップローダー、アップロードの同意 / 進行状況 / 結果ウィンドウ |
| `src/tray.rs`, `src/autostart.rs`, `src/single_instance.rs` | トレイアイコン、ログイン時起動、単一インスタンスガード |
| `src/snip.rs`, `src/settings.rs`, `src/paths.rs` | ギャラリーの永続化（SQLite + FTS）、設定、パス |
| `src/image_files.rs`, `src/atomic_file.rs` | ディスク上のスニップ画像: ロスレス WebP、アトミック書き込み |
| `src/i18n.rs`, `lang/` | 実行時 i18n + 組み込みの gettext `.po` カタログ（23 言語） |

</details>

---

## ✨ ReviseTouch 製

Snipdeck は **[ReviseTouch](https://revisetouch.com)** による無料のオープンソースアプリです ——
私たちは開発者ツールと AI 製品を開発しています。ほかに何を作っているかは
**[revisetouch.com](https://revisetouch.com)** をご覧ください。

---

## ライセンス

[MIT](LICENSE) © 2026 Hasan Salihoğlu

<div align="center">
<sub>🦀 Rust と <a href="https://slint.dev">Slint</a> で構築されています。</sub>
</div>
