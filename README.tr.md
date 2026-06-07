<div align="center">

[English](README.md) · **Türkçe** · [Español](README.es.md) · [简体中文](README.zh-CN.md) · [العربية](README.ar.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · [日本語](README.ja.md) · [Français](README.fr.md) · [Português](README.pt.md)

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**Windows için hızlı, klavye odaklı bir ekran kesimi _destesi_.**

Bir bölgeyi yakala, kesimleri her şeyin üstünde yüzer halde tut, üzerlerine
çizim yap, OCR ile dizinle ve yüzlerce ekran görüntüsünü tek bir yoğun galeriden
ara — hepsi, yazılımla render edilen [Slint](https://slint.dev) arayüzüne sahip
yerel bir Rust uygulamasından.

![platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows&logoColor=white)
![soon](https://img.shields.io/badge/macOS%20%2F%20Linux-on%20the%20way-lightgrey)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Slint-orange?logo=rust&logoColor=white)
![languages](https://img.shields.io/badge/languages-23-success)
![license](https://img.shields.io/badge/license-MIT-green)

</div>

---

## 📸 Ekran görüntüleri

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · translate.png
-->

<div align="center">

| Galeri çalışma alanı | Masaüstünde yüzen kesimler |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="Galeri" width="420" /> | <img src="docs/screenshots/floating.png" alt="Yüzen kesimler" width="420" /> |
| **İşaretleme editörü** | **OCR + çeviri** |
| <img src="docs/screenshots/annotate.png" alt="İşaretleme editörü" width="420" /> | <img src="docs/screenshots/translate.png" alt="OCR çeviri" width="420" /> |

</div>

---

## ✨ Öne çıkanlar

- 🖱️ **Bir değiştirici tuş + sürükleyerek yakalama.** `Win` tuşunu basılı tutup bir bölgeyi
  sürükle; sonucun ne olacağına değiştirici tuşlar karar verir — yüzen pencere, pano,
  yalnızca galeri ([matris aşağıda](#️-yakalama-kısayolları)).
- 🧊 **Önce dondur yakalama** (`Win`+`Shift`+`Space`). Seçim yapmadan *önce* ekranı karartıp
  dondurur; böylece üzerine gelince çıkan ipuçları ve tıklamaya duyarlı diğer arayüzler yakalanır.
- 📌 **Yüzen kesimler.** Yakalananlar; sürükleyebileceğin, yeniden boyutlandırabileceğin,
  sabitleyebileceğin, kırpabileceğin, döndürüp çevirebileceğin ve kapatabileceğin hafif,
  çerçevesiz katmanlar olarak üstte kalır.
- ✏️ **Dahili işaretleme editörü.** Kalem, fosforlu kalem, şekiller, oklar ve metin; tam bir HSV
  renk tekerleği, ayarlanabilir boyut + opaklık, yakınlaştırma ve geri al/yinele ile
  ([ayrıntılar](#️-işaretleme-editörü)).
- 🔍 **Dahili OCR.** Her kesim, işletim sistemi motoruyla (`Windows.Media.Ocr`) arka planda
  OCR'lanıp dizinlenir — model indirme yok, ağ yok, tamamen yerel.
- 🌐 **OCR + çeviri.** Bir kesimin metnini satır içinde başka bir dilde oku (30+ hedef,
  yıldızlayabileceğin favorilerle).
- 🗂️ **Güçlü galeri.** OCR metni, pencere meta verileri, monitör ve yol üzerinde tam metin
  araması; duruma / içeriğe / boyuta / tarihe göre filtreleme; binlerce kesimde bile akıcı
  kalan sanallaştırılmış bir ızgara.
- 📤 **Paylaş & yükle.** Windows paylaşım sayfası, yeni bir e-posta (Simple MAPI), sistemin
  görüntü düzenleyicisi veya bir görüntü barındırıcısı — yüklemeler **açık bir onay
  penceresiyle isteğe bağlıdır.**
- 🌍 **23 dil**, işletim sisteminden otomatik algılanır ve araç çubuğundan değiştirilebilir.
- 🌗 **Açık / koyu temalar**, sistem tepsisi yuvası ve **kayıt defteri kullanmayan**
  oturum açılışında başlatma.

---

## ⌨️ Yakalama kısayolları

`Win` tuşunu basılı tut, ardından **sol** fare düğmesiyle bir bölgeyi sürükle. Kesime ne
olacağını değiştirmek için değiştirici tuşlar ekle:

| Kısayol | Sonuç |
| --- | --- |
| `Win` + sürükle | Ekranda yüzen kesim |
| `Win`+`Ctrl` + sürükle | Yüzen kesim **ve** panoya kopyala |
| `Win`+`Alt` + sürükle | Yalnızca pano (galeriye eklenmez) |
| `Win`+`Shift` + sürükle | Yalnızca galeri (yüzen pencere yok) |
| `Win`+`Ctrl`+`Shift` + sürükle | Galeri **ve** pano, yüzen pencere yok |
| `Win`+`Shift`+`Space`, sonra sürükle | **Önce dondur** — ekranı dondur, sonra durağan görüntü üzerinde seç |

Ayrıca tepsi simgesine sağ tıklayıp → **New Snip** ile `Win` tuşunu basılı tutmadan tek
seferlik bir sürükleme yakalaması da hazırlayabilirsin.

### Genel kısayollar

`Win` + sürükle dışında Snipdeck, beş yapılandırılabilir genel kısayol tuşu da kaydeder:

| Kısayol | İşlem |
| --- | --- |
| `Win`+`Alt`+`H` | Ekrandaki tüm kesimleri göster / gizle |
| `Ctrl`+`Shift`+`A` | Yeni kesim |
| `Ctrl`+`Shift`+`C` | Yeni kesim **ve** panoya kopyala |
| `Ctrl`+`Shift`+`X` | Yalnızca pano |
| `Ctrl`+`Shift`+`G` | Galeriyi aç |

Bunları `settings.json` içindeki `hotkey_*` anahtarlarıyla değiştir.

---

## ✏️ İşaretleme editörü

Herhangi bir kesime sağ tıkla → **Annotate** ile kesimlerinin üstünde duran, Telegram tarzı
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

Yüzen bir kesime **veya** galeri küçük resmine sağ tıkla:

- **Copy** — görüntüyü panoya kopyala
- **OCR + Clipboard** — OCR çalıştır ve tanınan metni kopyala
- **OCR + Translate** — çeviri penceresini aç
- **Annotate** — yukarıdaki editörü aç
- **Share** — Windows paylaşım sayfası, **Mail** (MAPI eki), **editor**'de aç (Paint) veya
  bir görüntü barındırıcısına **Upload**
- **Transform** — döndür (90° / 180°), yatay / dikey çevir
- **Crop** (tutamaçları içeri sürükle) ve **Resize**
- **Hide from screen** / **Delete**

---

## 📦 Kurulum

### İndir

En güncel `snipdeck.exe` dosyasını [**Releases**](https://github.com/hasantr/snipdeck/releases)
sayfasından al. Tek bir taşınabilir çalıştırılabilir dosyadır — kurulum gerektirmez.

### Kaynaktan derle

Gereksinimler: [Rust](https://rustup.rs/) (stable) ve Windows 10 veya 11.

```powershell
git clone https://github.com/hasantr/snipdeck.git
cd snipdeck
cargo build --release
```

Çalıştırılabilir dosya `target\release\snipdeck.exe` konumunda oluşur.

---

## 🌍 Diller

Snipdeck tamamen yerelleştirilmiştir. İlk açılışta dilini işletim sistemi yerelinden seçer;
araç çubuğundaki **küre menüsünden** dilediğin zaman değiştirebilirsin.

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

- **Yakalama ve OCR tamamen yereldir.** OCR, dahili `Windows.Media.Ocr` motorunu kullanır;
  yakalama veya dizinleme için hiçbir şey dışarıya gönderilmez.
- **Oturum açılışında başlatma kayıt defteri kullanmaz.** Snipdeck, `HKCU\…\Run` değeri yazmak
  yerine `shell:Startup` klasörüne normal bir kısayol bırakır — bu, antivirüs / EDR araçlarının
  en sık işaretlediği otomatik başlatma kayıt defteri anahtarından kaçınan, kullanıcının
  görebildiği olağan yöntemdir.
- **Yüklemeler isteğe bağlıdır.** Hedefi açıkça belirten bir onay penceresi olmadan hiçbir şey
  makineni terk etmez. Hassas içeriği bağlantısı olana açık bir barındırıcıya yükleme.
- **OCR + Translate ağı kullanır.** Yalnızca OCR'lanan *metin* çevrilmek üzere Google'ın herkese
  açık `translate_a` uç noktasına gönderilir; başka hiçbir yerde kullanılmaz.

---

## 🗺️ Yol haritası

- [x] Windows 10 / 11
- [ ] **macOS** — yolda
- [ ] **Linux** — yolda
- [ ] Uygulama içi ayarlar penceresi
- [ ] Daha fazla işaretleme aracı (bulanıklaştır / pikselleştir, numaralı adım rozetleri, şekil taşıma/düzenleme)

> Snipdeck bugün yalnızca Windows içindir; çünkü yakalama, OCR, tepsi ve paylaşımın tümü
> Windows'a özgü API'ler kullanır. Arayüz ([Slint](https://slint.dev)) ve çekirdeğin büyük kısmı
> zaten çapraz platform olduğundan **macOS ve Linux sürümleri yolda.**

---

## ⚙️ Yapılandırma

Ayarlar JSON olarak saklanır ve ilk çalıştırmada oluşturulur. Henüz bir ayarlar penceresi yok —
dosyayı doğrudan düzenle, sonra Snipdeck'i yeniden başlat.

| Yol | İçerik |
| --- | --- |
| `%APPDATA%\Snipdeck\settings.json` | Kullanıcı ayarları (aşağıda) |
| `%LOCALAPPDATA%\Snipdeck\` | `snips.db`, `state.json` ve görüntü + küçük resimlerin `cache\` dizini |
| `Pictures\Snipdeck\` | *Save* için varsayılan klasör |

<details>
<summary><b>Seçili <code>settings.json</code> anahtarları</b></summary>

| Anahtar | Varsayılan | Anlamı |
| --- | --- | --- |
| `language` | `"auto"` | Arayüz dili kodu veya işletim sistemini izlemek için `"auto"` |
| `dark_theme` | `true` | Açık veya koyu kabuk |
| `upload_service` | `"catbox"` | *Upload* için görüntü barındırıcısı: `"catbox"` (anahtarsız) veya `"imgur"` |
| `imgur_client_id` | `""` | Imgur Client-ID, yalnızca `upload_service` `"imgur"` iken gerekli |
| `translate_target` | `"en"` | Varsayılan OCR-çeviri hedef dil kodu |
| `translate_favs` | `["en","es","fr","de","it","pt","ru","ar"]` | Çeviri penceresindeki hızlı seçim dilleri |
| `default_format` | `"png"` | *Save* için varsayılan uzantı |

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
| `src/mouse_trigger.rs`, `src/native_mouse_trigger.rs` | Kesimleri hazırlayıp süren düşük seviyeli fare/klavye kancaları |
| `src/slint_overlay.rs`, `ui/overlay.slint` | Tam ekran seçim katmanı (canlı veya dondurulmuş) |
| `src/floating.rs`, `ui/floating.slint` | Yüzen kesim pencereleri (sürükle, yeniden boyutlandır, kırp, kenarlık) |
| `src/annotate.rs`, `ui/annotate.slint` | İşaretleme editörü (tiny-skia düzleştirme, HSV tekerleği, yakınlaştırma) |
| `src/ocr.rs` | `Windows.Media.Ocr` sarmalayıcısı (bir iş parçacığında) |
| `src/translate.rs`, `src/translate_popup.rs` | OCR-çeviri penceresi |
| `src/share.rs`, `src/upload.rs` | Paylaşım sayfası / MAPI posta / düzenleyici / görüntü barındırıcısına yükleme |
| `src/tray.rs`, `src/autostart.rs`, `src/single_instance.rs` | Tepsi simgesi, oturum açılışında başlatma, tek örnek koruması |
| `src/snip.rs`, `src/settings.rs`, `src/paths.rs` | Galeri kalıcılığı (SQLite + FTS), ayarlar, yollar |
| `src/i18n.rs`, `lang/` | Çalışma zamanı i18n + gömülü gettext `.po` kataloğu (23 dil) |

</details>

---

## Lisans

[MIT](LICENSE) © 2026 Hasan Salihoğlu

<div align="center">
<sub>🦀 Rust ve <a href="https://slint.dev">Slint</a> ile yapıldı.</sub>
</div>
