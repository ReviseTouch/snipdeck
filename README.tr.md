<div align="center">

[English](README.md) · **Türkçe** · [Español](README.es.md) · [简体中文](README.zh-CN.md) · [العربية](README.ar.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · [日本語](README.ja.md) · [Français](README.fr.md) · [Português](README.pt.md)

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**Windows için hızlı, klavye odaklı bir ekran kırpma _destesi_.**

<sub><a href="https://revisetouch.com"><b>ReviseTouch</b></a> tarafından geliştirilen ücretsiz ve açık kaynaklı bir uygulama.</sub>

Bir bölgeyi yakala, snip'leri her şeyin üstünde yüzer halde tut, üzerlerine
çizim yap, OCR ile dizinle ve yüzlerce ekran görüntüsünü tek bir yoğun galeriden
ara — hepsi, yazılımla render edilen [Slint](https://slint.dev) arayüzüne sahip
yerel bir Rust uygulamasından.

![platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows&logoColor=white)
![linux](https://img.shields.io/badge/Linux-GNOME%20full%20%2F%20others%20partial-yellow) ![soon](https://img.shields.io/badge/macOS-on%20the%20way-lightgrey)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Slint-orange?logo=rust&logoColor=white)
![languages](https://img.shields.io/badge/languages-23-success)
![license](https://img.shields.io/badge/license-MIT-green)

</div>

---

## 📸 Ekran görüntüleri

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · collage.png
-->

<div align="center">

| Galeri çalışma alanı | Masaüstünde yüzen snip'ler |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="Galeri" width="420" /> | <img src="docs/screenshots/floating.png" alt="Yüzen snip'ler" width="420" /> |
| **İşaretleme editörü** | **Kolaj editörü** |
| <img src="docs/screenshots/annotate.png" alt="İşaretleme editörü" width="420" /> | <img src="docs/screenshots/collage.png" alt="Kolaj editörü" width="420" /> |

</div>

---

## ✨ Öne çıkanlar

- 🖱️ **Bir değiştirici tuş + sürükleyerek yakalama.** `Win` tuşunu basılı tutup bir bölgeyi
  sürükle; sonucun ne olacağına değiştirici tuşlar karar verir — yüzen pencere, pano,
  yalnızca galeri ([matris aşağıda](#️-yakalama-kısayolları)).
- 🧊 **Önce dondur yakalama** (`Win`+`Shift`+`Space`). Seçim yapmadan *önce* ekranı karartıp
  dondurur; böylece üzerine gelince çıkan ipuçları ve tıklamaya duyarlı diğer arayüzler yakalanır.
- 📌 **Yüzen snip'ler.** Yakalananlar; sürükleyebileceğin, yeniden boyutlandırabileceğin,
  sabitleyebileceğin, kırpabileceğin, döndürüp çevirebileceğin ve kapatabileceğin hafif,
  çerçevesiz katmanlar olarak üstte kalır.
- ✏️ **Dahili işaretleme editörü.** Kalem, fosforlu kalem, şekiller, oklar ve metin; tam bir HSV
  renk tekerleği, ayarlanabilir boyut + opaklık, yakınlaştırma ve geri al/yinele ile
  ([ayrıntılar](#️-i̇şaretleme-editörü)).
- 🧩 **Kolaj.** Birkaç snip'i tek bir görüntüde düzenle — ızgara, iki yana yaslı satırlar ya da
  tuğla dizilimi (masonry); boşluk, yuvarlak köşeler ve arka planla — sonuç galeriye yeni bir
  snip olarak kaydedilir.
- 🔍 **Dahili OCR.** Her snip, işletim sistemi motoruyla (`Windows.Media.Ocr`) arka planda
  OCR'lanıp dizinlenir — model indirme yok, ağ yok, tamamen yerel.
- 🌐 **OCR + çeviri.** Bir snip'in metnini satır içinde başka bir dilde oku (29 hedef,
  yıldızlayabileceğin favorilerle).
- 🗂️ **Güçlü galeri.** OCR metni, pencere meta verileri, monitör ve yol üzerinde tam metin
  araması; duruma / içeriğe / boyuta / tarihe göre filtreleme; binlerce snip'te bile akıcı
  kalan sanallaştırılmış bir ızgara ([tur](#️-galeri)).
- 📤 **Paylaş & yükle.** Windows paylaşım sayfası, yeni bir e-posta (Simple MAPI), sistemin
  görüntü düzenleyicisi veya bir görüntü barındırıcısı — catbox.moe, Imgur ya da bir ShareX
  `.sxcu` dosyasından **kendi sunucun** ([ayrıntılar](#-kendi-görüntü-barındırıcın)). Her yükleme **önce sorar.**
- 🌍 **23 dil**, işletim sisteminden otomatik algılanır ve araç çubuğundaki `⋯` menüsünden
  değiştirilebilir.
- 🌗 **Açık / koyu temalar**, sistem tepsisi yuvası ve **kayıt defteri kullanmayan**
  oturum açılışında başlatma.

---

## ⌨️ Yakalama kısayolları

`Win` tuşunu basılı tut, ardından **sol** fare düğmesiyle bir bölgeyi sürükle. Snip'e ne
olacağını değiştirmek için değiştirici tuşlar ekle:

| Kısayol | Sonuç |
| --- | --- |
| `Win` + sürükle | Ekranda yüzen snip |
| `Win`+`Ctrl` + sürükle | Yüzen snip **ve** panoya kopyala |
| `Win`+`Alt` + sürükle | Yalnızca pano (galeriye eklenmez) |
| `Win`+`Shift` + sürükle | Yalnızca galeri (yüzen pencere yok) |
| `Win`+`Ctrl`+`Shift` + sürükle | Galeri **ve** pano, yüzen pencere yok |
| `Win`+`Shift`+`Space`, sonra sürükle | **Önce dondur** — ekranı dondur, sonra durağan görüntü üzerinde seç |

Ayrıca tepsi simgesine sağ tıklayıp → **Yeni snip** ile `Win` tuşunu basılı tutmadan tek
seferlik bir sürükleme yakalaması da hazırlayabilirsin.

### Genel kısayollar

`Win` + sürükle dışında Snipdeck, beş yapılandırılabilir genel kısayol tuşu da kaydeder:

| Kısayol | İşlem |
| --- | --- |
| `Win`+`Alt`+`H` | Ekrandaki tüm snip'leri göster / gizle |
| `Ctrl`+`Alt`+`Shift`+`A` | Yeni snip |
| `Ctrl`+`Alt`+`Shift`+`C` | Yeni snip **ve** panoya kopyala |
| `Ctrl`+`Alt`+`Shift`+`X` | Yalnızca pano |
| `Ctrl`+`Alt`+`Shift`+`G` | Galeriyi aç |

Bunları `settings.json` içindeki `hotkey_*` anahtarlarıyla değiştir. Wayland'de genel
kısayollar masaüstünün elindedir: bunlar yalnızca ilk açılışta önerilir, sonrasında onları
masaüstünün klavye ayarlarından değiştirirsin.

---

## ✏️ İşaretleme editörü

Herhangi bir snip'e sağ tıkla → **İşaretle** ile snip'lerinin üstünde duran, Telegram tarzı
odaklı bir editör açılır.

- **Araçlar** — serbest kalem, fosforlu kalem, dikdörtgen, elips, çizgi, ok ve metin.
- **Tam HSV renk tekerleği** ve parlaklık kaydırıcısı — sabit bir palet değil, *istediğin* rengi seç.
- **Boyut & opaklık kaydırıcıları**, görüntü üzerinde yanıp sönen bir boyut önizlemesiyle;
  her araç için ayrı hatırlanır.
- Hassas çalışma için **yakınlaştır / uzaklaştır / sığdır**; yakınlaştırınca tuval kaydırılabilir.
- **Geri al / yinele / temizle**, ayrıca `Esc`, `Ctrl`+`Z`, `Ctrl`+`Y`.
- **Kaydet**'te çizimler görüntüye tam çözünürlükte işlenir (net, gerçek boyutta).

Canlı önizleme ile kaydedilen sonuç aynı koddan gelir; yani **ne görüyorsan onu alırsın.**

---

## 🖱️ Sağ tık işlemleri

Yüzen bir snip'e **veya** galeri küçük resmine sağ tıkla:

- **Kopyala** — görüntüyü panoya kopyalar
- **OCR + Pano** — OCR çalıştır ve tanınan metni kopyala
- **OCR + Çevir** — çeviri penceresini aç
- **İşaretle** — yukarıdaki editörü aç
- **Paylaş** — Windows paylaşım sayfası, **E-posta** (MAPI eki), **düzenleyicide** aç (Paint)
  veya bir görüntü barındırıcısına **Yükle** ([hangisine](#-kendi-görüntü-barındırıcın))
- **Dönüştür** — döndür (90° / 180°), yatay / dikey çevir
- **Kırp** (tutamaçları içeri sürükle) ve **Yeniden boyutlandır**
- **Ekrandan gizle** / **Sil**

---

## 🗂️ Galeri

Tek bir araç çubuğu satırı; geri kalan her şey bir tık uzakta.

- **Yeni** bir snip alır. Okunun altında **Snip ve kopyala** ile **Yalnız panoya** durur; her
  biri kendi kısayoluyla.
- **Kolaj** ve **Renk seç** hemen yanındadır; satırın geri kalanını arama kutusu kaplar —
  odaklamak için `Ctrl`+`F`, temizlemek için `Esc`.
- **Filtrele** durum, içerik, boyut ve tarihi tek bir panelde açar. Etkin filtreler araç
  çubuğunun altında çip olarak görünür; her biri tek tek ya da **Filtreleri temizle** ile
  hepsi birden kaldırılabilir.
- **`⋯`** seyrek değişen şeyleri tutar: Yüzen snip'ler görev çubuğunda, Koyu tema, Oturum
  açılınca başlat, Dil, galeriyi temizleme ve Snipdeck Hakkında.
- Durum çubuğu gösterilenleri sayar; **Sırala** ve **S / M / L** kart boyutu da oradadır.

Hızlı işlemleri için bir kartın üzerine gel; [geri kalan her şey](#️-sağ-tık-işlemleri) için
sağ tıkla.

---

## 📤 Kendi görüntü barındırıcın

**`⋯` → Yükleme yeri**, **Yükle**'nin bir snip'i nereye göndereceğini seçer: catbox.moe (varsayılan,
hesap gerekmez), Imgur ya da kendi barındırıcın. Eklemek için **.sxcu dosyasından ekle…** seçeneğini
seç ve bir ShareX özel yükleyici dosyası göster — Zipline, XBackBone, Chevereto ve kendi kendine
barındırılan görüntü sunucularının çoğu bu biçimi zaten sunar, yani elle yazılacak bir şey yok.

Snipdeck görüntüyü multipart form verisi ya da ikili gövde olarak, dosyadaki başlıklar, argümanlar
ve sorgu parametreleriyle gönderir; bağlantıyı `{json:…}`, `{regex:…}`, `{header:…}` veya
`{response}` ile okur — ShareX 12'nin `$json:…$` biçimiyle de. Daha fazlasını gerektiren bir dosya
(bir giriş penceresi, rastgele bir değer, XML yanıtı, görüntü barındırıcısı olmayan bir hedef) ilk
yüklemede başarısız olmak yerine, içe aktarırken gerekçesiyle birlikte reddedilir.

Bir şey gönderilmeden önce Snipdeck'in kendi penceresi barındırıcıyı ve adresini söyler, yüklemenin
ne anlama geldiğini açıklar; iş bitince bağlantıyı gösterir ve onu çoktan kopyalamış olur.

---

## 📦 Kurulum

### İndir

**Windows** — `snipdeck-windows-x64.exe` dosyasını
[**Releases**](https://github.com/ReviseTouch/snipdeck/releases) sayfasından al. Tek bir
taşınabilir çalıştırılabilir dosya; kurulum gerektirmez.

**Linux (x64)** — `snipdeck-linux-x64.tar.gz` dosyasını aynı sayfadan al:

```bash
tar xzf snipdeck-linux-x64.tar.gz
cd snipdeck-linux-x64
./install.sh          # into ~/.local, no root; --uninstall reverses it
```

Ardından uygulamayı uygulamalar menünden başlat. Bu bir tarz tercihi değil: Wayland'de
masaüstü portalı, uygulama kimliğini uygulamanın başlatıldığı `.desktop` girdisinden türetir
ve böyle bir girdi yoksa hiçbir genel kısayolu bağlamayı kabul etmez.

### Kaynaktan derle

Gereksinimler: [Rust](https://rustup.rs/) (stable).

```bash
git clone https://github.com/ReviseTouch/snipdeck.git
cd snipdeck
cargo build --release
```

**Windows** (10 veya 11): çalıştırılabilir dosya `target\release\snipdeck.exe` konumunda oluşur.

**Linux**: derleme için GTK, X11/Wayland, tepsi, D-Bus ve GL başlık dosyaları gerekir.
Debian/Ubuntu'da:

```bash
sudo apt install libgtk-3-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxcb-randr0-dev libxrandr-dev libxkbcommon-dev \
  libwayland-dev libayatana-appindicator3-dev libfontconfig-dev libdbus-1-dev \
  libgl1-mesa-dev libegl1-mesa-dev
```

PipeWire derleme sırasında hiçbir şey gerektirmez: Wayland yakalaması için çalışma zamanında
yüklenir. Kendi derlemeni bir sürümün kurulduğu şekilde kurmak için onu kurucunun yanına
yerleştir:

```bash
packaging/linux/install-from-source.sh   # builds, stages afresh, installs
```

---

## 🌍 Diller

Snipdeck tamamen yerelleştirilmiştir. İlk açılışta dilini işletim sistemi yerelinden seçer;
araç çubuğundaki **`⋯` → Dil** ile dilediğin zaman değiştirebilirsin.

> English · Español · 简体中文 · हिन्दी · العربية · Português · Русский · 日本語 · Deutsch ·
> Français · 한국어 · Italiano · Türkçe · Indonesia · Tiếng Việt · Polski · Українська ·
> Nederlands · فارسی · ไทย · বাংলা · اردو · Română

Uygulamanın tamamı — hem Slint arayüzü (`@tr`) hem de Rust tarafındaki mesajlar (`i18n::t`) —
`lang/<kod>/LC_MESSAGES/snipdeck.po` altındaki **dil başına tek bir gettext `.po`** dosyasından
okunur ve derleme sırasında ikiliye gömülür (çalışma zamanında gettext bağımlılığı yoktur).

<details>
<summary><b>Çeviri ekleme veya güncelleme</b></summary>

1. Yeni dizeler `@tr("…")` (`.slint` içinde) veya `i18n::t("…")` (Rust içinde) ile sarılır.
2. `lang/snipdeck.pot`, her İngilizce dizeyi listeleyen şablondur.
3. Yeni bir msgid'yi her `lang/<kod>/LC_MESSAGES/snipdeck.po` dosyasına kopyalayıp `msgstr`
   alanını doldur — yer tutucuları (`{}`, `{e}`, `{url}`, `{path}`) ve `\r\n` kaçışlarını koru.
4. Yeni bir dil eklemek için `.po` dosyasını oluştur ve kodu `src/i18n.rs` içindeki `LANGUAGES`
   ve `po_source()`'a ekle.

</details>

---

## 🔒 Gizlilik & güvenlik

- **Yakalama ve OCR yereldir.** OCR, Windows'ta `Windows.Media.Ocr`, Linux'ta ise
  Tesseract kullanır (bir boru üzerinden beslenir, geçici dosya yok); yakalama veya dizinleme
  için hiçbir şey hiçbir yere gönderilmez. Wayland'de ekran ScreenCast portalı üzerinden
  okunur: izni bir kez verirsin ve paylaşım göstergesi yalnızca bir snip alındığı an görünür.
- **OCR + Çevir önce sorar.** İlk seferde, OCR'lanan *metnin* — asla görüntünün değil —
  Google Translate'e gideceğini açıklar ve onayını bekler. Metin URL'de değil, istek
  gövdesinde gönderilir.
- **Yüklemeler her seferinde önce sorar.** Barındırıcıyı ve adresini söyleyen ve bunun ne anlama
  geldiğini açıklayan bir pencerede **Yükle**'ye basana kadar hiçbir şey makineni terk etmez:
  bağlantıya sahip herkes görüntüyü açabilir, catbox'a yüklenen bir şey silinemez, düz `http://`
  kullanan bir sunucu onu şifrelemeden gönderir. Hassas içeriği bağlantısı olana açık bir
  barındırıcıya yükleme. İçe aktarılan bir `.sxcu` dosyasındaki API anahtarları, ShareX'in yaptığı
  gibi `settings.json` içinde saklanır.
- **Devredilen kopyalar gizli kalır.** Bir e-posta istemcisine veya düzenleyiciye aktarılan
  görüntüler yalnızca senin okuyabildiğin bir klasöre gider (Linux'ta `$XDG_RUNTIME_DIR`,
  Windows'ta `%TEMP%\Snipdeck`) ve ardından temizlenir.
- **Günlükler iz bırakmaz.** İşaretçi koordinatları ve pencere başlıkları yalnızca hata
  ayıklama düzeyinde günlüğe yazılır.
- **Oturum açılışında başlatma dosya tabanlıdır.** Windows'ta `shell:Startup` içinde normal
  bir kısayol (güvenlik araçlarının en sık işaretlediği `HKCU\…\Run` anahtarı değil), Linux'ta
  bir XDG autostart girdisi.

---

## 🗺️ Yol haritası

- [x] Windows 10 / 11
- [x] **Linux / GNOME** (Wayland) — birlikte gelen kabuk uzantısıyla: `Super`+`Alt`+sürükle
      veya kısayol tuşlarıyla yakalama, yakalandıkları yere yerleştirilip üstte tutulan
      snip'ler, OCR, renk seçici
- [~] **Linux / diğer masaüstleri** — kısayol tuşlarından ve araç çubuğundan yakalama,
      OCR, renk seçici (masaüstü portalı üzerinden). GNOME dışında değiştirici+sürükle
      hareketi yok: bunun için genel bir işaretçi kancası gerekir ve Linux bunu yalnızca
      `/dev/input` okunarak verir — Snipdeck'in senden etkinleştirmeni istemeyeceği, sistem
      genelinde bir tuş kaydedici. Wayland'de uzantı olmadan yerleştirme ve her zaman üstte
      tutma eksiktir (aşağıya bak)
- [ ] **macOS** — yolda
- [ ] Uygulama içi ayarlar penceresi
- [ ] Daha fazla işaretleme aracı (bulanıklaştır / pikselleştir, numaralı adım rozetleri, şekil taşıma/düzenleme)

### Linux'ta çalıştırma

Uygulamayı kur ve bir terminalden değil, **uygulamalar menüsünden başlat**. Genel kısayollar
masaüstü portalı üzerinden geçer; portal, uygulama kimliği olmayan bir istemcinin oturumunu
reddeder ve korumalı alan (sandbox) dışında çalışan bir uygulama bu kimliği yalnızca masaüstü
onu bir `.desktop` girdisinden başlattığında alır — bkz. `packaging/linux/snipdeck.desktop`.

Çalışma zamanı bağımlılıkları: `libayatana-appindicator` (tepsi), `xdg-utils` (paylaşım) ve
`tesseract` ile istediğin dil verileri (OCR — isteğe bağlı; yoksa OCR hata vermek yerine
atlanır).

**Wayland'in izin vermedikleri.** Bir Wayland istemcisi kendi pencerelerini konumlandıramaz,
nerede olduklarını soramaz ya da diğerlerinin üstünde kalmalarını isteyemez; üçü de
bileştiricinin (compositor) elindedir. Bu yüzden yalın bir Wayland oturumunda bir snip,
yakalandığı bölgenin üstünde değil, bileştirici onu nereye koyarsa orada açılır; diğer
uygulamaların üstünde yüzemez ve pencere listesinde görünür. Geri kalan her şey — bölge
yakalama, dondurma katmanı, genel kısayollar, OCR, sürükleme, sağ tık menüsü — çalışır.

KDE ve wlroots tabanlı bileştiriciler (sway, Hyprland, river, COSMIC), tam olarak bu üçünü
sağlayan `wlr-layer-shell`'i sunar; Snipdeck henüz bunu kullanmıyor. GNOME/Mutter ise bunu
uygulamayı reddetti.

**GNOME'da açığı bir kabuk uzantısı kapatır.** Bir istemcinin yapamadığını, GNOME Shell'in
içinde çalışan kod yapabilir. Bu yüzden `install.sh`, bir GNOME oturumu algıladığında
`snipdeck@revisetouch.com` uzantısını da (`packaging/gnome-extension/` içinden) kurar —
Snipdeck'in snip pencerelerini izleyip onları yakalandıkları yere yerleştiren, diğer
pencerelerin üstünde tutan, her çalışma alanına sabitleyen ve pencere değiştiriciden çıkaran
tek bir dosya. Konum bilgisi pencere başlığında taşınır; bu çerçevesiz pencerelerde başlık
hiçbir zaman görünmez.

Uzantı yakalama hareketini de geri getirir. GNOME'da `Super`+sürükle zaten pencereleri
taşıdığı için hareket **`Super`+`Alt`+sürükle**'dir ve Snipdeck çalıştığı sürece etkindir:

| Hareket (GNOME) | Sonuç |
|---|---|
| `Super`+`Alt` + sürükle | Ekranda yüzen snip |
| `Super`+`Alt`+`Ctrl` + sürükle | Yüzen snip **ve** panoya kopyala |
| `Super`+`Alt`+`Shift` + sürükle | Yalnızca galeri (yüzen pencere yok) |
| `Super`+`Alt`+`Ctrl`+`Shift` + sürükle | Galeri **ve** pano, yüzen pencere yok |

"Yalnızca pano"nun — Windows'ta `Win`+`Alt` — burada bir hareketi yok; onun kısayol tuşunu
kullan. `Esc` bir sürüklemeyi iptal eder; `Super`+`Alt`'ı basılı tutmaya devam edersen arka
arkaya birkaç snip alabilirsin.

Kurduktan sonra oturumu kapatıp yeniden aç; GNOME Shell, Wayland'de uzantıları yerinde
yeniden yükleyemez, çünkü oturumunu taşıyan bileştiricinin *kendisidir*. Uzantı tamamen
isteğe bağlıdır — o olmadan Snipdeck diğer Wayland istemcileri gibi davranır — ve
Snipdeck'in kendi pencereleri dışında yalnızca `Super`+`Alt`+sürükle'ye dokunur, o da
yalnızca Snipdeck çalışırken.

GNOME Wayland'de hâlâ eksik olan: sürüklediğin bir snip'i Mutter taşır ve uygulamaya nereye
bırakıldığı hiç bildirilmez; bu yüzden konumu yeniden başlatmalar arasında hatırlanmaz.

---

## ⚙️ Yapılandırma

Ayarlar JSON olarak saklanır ve ilk çalıştırmada oluşturulur. Henüz bir ayarlar penceresi yok —
dosyayı doğrudan düzenle, sonra Snipdeck'i yeniden başlat.

| Yol | İçerik |
| --- | --- |
| `%APPDATA%\Snipdeck\settings.json` | Kullanıcı ayarları (aşağıda) |
| `%LOCALAPPDATA%\Snipdeck\` | `snips.db`, `state.json` ve görüntü + küçük resimlerin `cache\` dizini |
| `Pictures\Snipdeck\` | *Kaydet* için varsayılan klasör |

<details>
<summary><b>Seçili <code>settings.json</code> anahtarları</b></summary>

| Anahtar | Varsayılan | Anlamı |
| --- | --- | --- |
| `language` | `"auto"` | Arayüz dili kodu veya işletim sistemini izlemek için `"auto"` |
| `dark_theme` | `true` | Açık veya koyu kabuk |
| `upload_service` | `"catbox"` | *Yükle* için görüntü barındırıcısı: `"catbox"` (anahtarsız), `"imgur"` veya `"custom:<name>"`; `⋯` → Yükleme yeri menüsünden seçilir |
| `upload_hosts` | `[]` | ShareX `.sxcu` dosyalarından içe aktarılan barındırıcılar, ShareX'in kendi alan isimleriyle |
| `imgur_client_id` | `""` | Imgur Client-ID, yalnızca `upload_service` `"imgur"` iken gerekli |
| `translate_target` | `"en"` | Varsayılan OCR-çeviri hedef dil kodu |
| `translate_favs` | `["en","es","fr","de","it","pt","ru","ar"]` | Çeviri penceresindeki hızlı seçim dilleri |
| `default_format` | `"png"` | *Kaydet* için varsayılan uzantı |

**Yükleme servisleri** — *catbox.moe* (varsayılan) hesap veya anahtar gerektirmez, ama anonim
yüklemeler sonradan **senin tarafından silinemez.** *Imgur* ücretsiz bir Client-ID gerektirir
(imgur.com → Settings → Applications adresinden bir kez kaydol). Her iki durumda da görüntü
listelenmez ama bağlantıya sahip herkes açabilir ve şifrelenmez.

</details>

---

## 🧩 Mimari

Arayüzü tek iş parçacıklı bir Slint olay döngüsü sürer; işletim sistemi kancaları ve bloklayan
işler kendi iş parçacıklarında çalışıp sonuçları kanallar üzerinden geri verir ve tik başına bir
kez boşaltılır.

<details>
<summary><b>Modül haritası</b></summary>

| Modül | Sorumluluk |
| --- | --- |
| `src/app/` | Uygulama durumu, tik başına döngü, arayüz eylemleri, galeri modeli |
| `src/capture.rs`, `src/dxgi_capture.rs` | Monitör yakalama (DXGI Desktop Duplication, GDI yedeği) |
| `src/mouse_trigger.rs`, `src/native_mouse_trigger.rs` | Snip'leri hazırlayıp süren düşük seviyeli fare/klavye kancaları |
| `src/slint_overlay.rs`, `ui/overlay.slint` | Tam ekran seçim katmanı (canlı veya dondurulmuş) |
| `src/floating.rs`, `ui/floating.slint` | Yüzen snip pencereleri (sürükle, yeniden boyutlandır, kırp, kenarlık) |
| `src/annotate.rs`, `ui/annotate.slint` | İşaretleme editörü (tiny-skia düzleştirme, HSV tekerleği, yakınlaştırma) |
| `src/ocr.rs` | `Windows.Media.Ocr` sarmalayıcısı (bir iş parçacığında) |
| `src/translate.rs`, `src/translate_popup.rs` | OCR-çeviri penceresi |
| `src/share.rs`, `src/upload.rs` | Paylaşım sayfası / MAPI posta / düzenleyici / görüntü barındırıcısına yükleme |
| `src/sxcu.rs`, `src/upload_window.rs`, `ui/upload.slint` | ShareX özel yükleyicileri; yükleme onayı / ilerleme / sonuç penceresi |
| `src/tray.rs`, `src/autostart.rs`, `src/single_instance.rs` | Tepsi simgesi, oturum açılışında başlatma, tek örnek koruması |
| `src/snip.rs`, `src/settings.rs`, `src/paths.rs` | Galeri kalıcılığı (SQLite + FTS), ayarlar, yollar |
| `src/image_files.rs`, `src/atomic_file.rs` | Diskteki snip görüntüleri: kayıpsız WebP, atomik yazma |
| `src/i18n.rs`, `lang/` | Çalışma zamanı i18n + gömülü gettext `.po` kataloğu (23 dil) |

</details>

---

## ✨ ReviseTouch tarafından yapıldı

Snipdeck, **[ReviseTouch](https://revisetouch.com)** tarafından geliştirilen ücretsiz, açık
kaynaklı bir uygulamadır — geliştirici araçları ve yapay zekâ ürünleri geliştiriyoruz. Başka
neler geliştirdiğimizi görmek için **[revisetouch.com](https://revisetouch.com)** adresine göz at.

---

## Lisans

[MIT](LICENSE) © 2026 Hasan Salihoğlu

<div align="center">
<sub>🦀 Rust ve <a href="https://slint.dev">Slint</a> ile yapıldı.</sub>
</div>
