---
title: "Snipdeck — Windows için hızlı, klavye odaklı bir ekran kırpma destesi"
description: "ReviseTouch'ın Windows için ücretsiz, açık kaynaklı Rust + Slint ekran kırpma uygulaması Snipdeck'in açılış sayfası metni ve bölüm spesifikasyonu."
order: 1
category: "Ürün"
---

# Snipdeck açılış sayfası — metin + bölüm spesifikasyonu

Bu belge, revisetouch.com üzerindeki `/snipdeck` ürün sayfasının **metin + bölüm spesifikasyonudur**. Mevcut Evreka/Çolpan ürün sayfasının bölüm ritmini ve tonunu yansıtır (hero rozeti → "Nedir" → özellik kartları → teknik/spesifikasyon tabloları → hedef kitle/güven → son CTA). Bitmiş bir `.astro` dosyası değildir; aşağıdaki her bölüm; bölüm adını, üst başlık/başlığı, gövde metnini, varsa kartları, bağlantı hedefleriyle birlikte buton etiketlerini ve tek satırlık bir yerleşim notunu verir.

Buradaki her iddia `README.md` dosyasına dayanır. Hiçbir şey uydurulmamıştır. README'nin temkinli davrandığı yerlerde (ör. "yolda") metin de temkinli davranır.

**Boyunca kullanılan genel bağlantı hedefleri:**

| Belirteç | Hedef |
| --- | --- |
| `DOWNLOAD` | `https://github.com/hasantr/snipdeck/releases` (en güncel `snipdeck.exe`) |
| `DOCS` | `/docs/snipdeck` |
| `GITHUB` | `https://github.com/hasantr/snipdeck` |
| `REVISETOUCH` | `https://revisetouch.com` |

---

## 1. Hero

- **Bölüm adı:** `hero`
- **Üst başlık / rozet:** `ÜCRETSİZ · AÇIK KAYNAK · WINDOWS 10 / 11`
- **Başlık (1. satır):** Bir kez yakala.
- **Başlık (2. satır, vurgu):** Üstte tut, üzerine işaretle, sonra ara.
- **Alt başlık / giriş:**
  > Snipdeck, Windows için hızlı, klavye odaklı bir ekran kırpma destesidir. `Win` tuşunu basılı tutup bir bölge sürükleyin — kırpmanız her şeyin üstünde yüzer, arka planda OCR ile dizinlenir ve yoğun, aranabilir bir galeriye düşer. Yerel (native) Rust, yazılımla işlenen bir Slint arayüzü, hesap yok, kurulum yok.
- **Güven şeridi (giriş altındaki küçük tek aralıklı çipler):** `Rust + Slint` · `Yerel OCR` · `23 dil` · `MIT lisanslı` · `Tek taşınabilir .exe`
- **Birincil CTA butonu:** **Windows için indir** → `DOWNLOAD`
- **İkincil CTA butonu:** **Belgeleri oku** → `DOCS`
- **Üçüncül bağlantı (metin):** **Kaynağı GitHub'da görüntüle** → `GITHUB`
- **Yerleşim notu:** Fareyi takip eden nokta matrisli ızgara arka planı üzerinde ortalanmış hero (Evreka ile aynı bileşen); yan yana iki buton, altında soluk bir GitHub metin bağlantısı, ardından galerinin çerçeveli bir uygulama maketi.

---

## 2. Snipdeck nedir

- **Bölüm adı:** `what`
- **Üst başlık:** Nedir
- **Başlık:** Ekran görüntülerini unutmayan bir kırpma aracı.
- **Gövde metni:**
  > Çoğu kırpma aracı bir resim çeker ve çekip gider. Snipdeck her yakalamayı, geri döneceğiniz bir şey olarak ele alır. Kırpmalar, siz çalışırken hafif yüzen pencereler olarak üstte sabit kalır, her biri alındığı anda yerel olarak OCR ile dizinlenir ve tüm geçmiş, binlerce kırpmada tam metin araması yapabileceğiniz tek bir sanallaştırılmış galeride yaşar.
  >
  > Rust ile yazılmış, yazılımla işlenen bir Slint arayüzüne sahip tek bir taşınabilir yürütülebilir dosyadır — kurulum yok, yönetilecek bir çalışma zamanı yok ve yakalama ile OCR tamamen makinenizde çalışır.
- **Öne çıkan vurgu kartı:**
  > `Win` tuşunu basılı tutun, bir bölge sürükleyin, bırakın. Gerisine değiştirici tuşlar karar verir — onu yüzdürün, kopyalayın veya doğrudan galeriye gönderin.
- **Yerleşim notu:** İki sütunlu bölünmüş düzen (Evreka "what" deseni): solda metin + vurgu kartı, sağda OCR ile okunmuş metinle eşleşen bir galeri aramasını gösteren küçük bir terminal/konsol tarzı maket.

---

## 3. Temel özellikler (kartlar)

- **Bölüm adı:** `features`
- **Başlık:** Bir ekran görüntüsünün hak ettiği her şey
- **Alt başlık:** Tek bir yakalama hareketi, ardından sonrasında olacaklar için eksiksiz bir araç seti.
- **Kartlar:**

| Kart başlığı | Tek satırlık açıklama |
| --- | --- |
| **`Win` + sürükle ile yakala** | `Win` tuşunu basılı tutun ve bir bölge sürükleyin; değiştirici tuşlar kırpmayı yüzen bir pencereye, panoya, galeriye veya hepsine birden gönderir. |
| **Önce dondur, sonra yakala** | `Win`+`Shift`+`Space`, seçim yapmadan *önce* ekranı karartıp dondurur; böylece araç ipuçları ve yalnızca üzerine gelince görünen arayüz yakalanmış olarak kalır. |
| **Yüzen kırpmalar** | Yakalamalar; sürükleyebileceğiniz, yeniden boyutlandırabileceğiniz, sabitleyebileceğiniz, kırpabileceğiniz, döndürebileceğiniz, çevirebileceğiniz ve kapatabileceğiniz çerçevesiz katmanlar olarak üstte sabit kalır. |
| **Açıklama düzenleyici** | Kalem, fosforlu kalem, şekiller, oklar ve metin; tam HSV renk tekerleği, boyut ve opaklık kaydırıcıları, yakınlaştırma ve geri al/yinele ile. |
| **Yerleşik OCR** | Her kırpma, işletim sistemi motoru (`Windows.Media.Ocr`) tarafından arka planda OCR ile dizinlenir — model indirme yok, ağ yok, tamamen yerel. |
| **OCR + çeviri** | Bir kırpmanın metnini başka bir dilde satır içinde okuyun; 30'dan fazla hedef ve yıldızlayabileceğiniz favorilerle. |
| **Güçlü galeri** | OCR metni, pencere meta verileri, monitör ve yol üzerinde tam metin araması; sanallaştırılmış bir ızgarada duruma, içeriğe, boyuta ve tarihe göre filtreleyin. |
| **Paylaş ve yükle** | Windows paylaşım sayfasına, yeni bir e-postaya (Simple MAPI), sistem görüntü düzenleyicisine veya bir görüntü barındırıcısına gönderin — yüklemeler isteğe bağlıdır. |

- **Yerleşim notu:** Üç sütunlu, çerçeveli kart ızgarası (Evreka `features` deseni); her kartta bir simge döşemesi, kalın başlık ve tek satırlık açıklama.

---

## 4. Yakalama kısayolu matrisi

- **Bölüm adı:** `shortcuts`
- **Üst başlık:** Kas hafızası
- **Başlık:** Tek hareket, altı sonuç
- **Gövde metni:**
  > `Win` tuşunu basılı tutun, ardından **sol** fare düğmesiyle bir bölge sürükleyin. Kırpmaya ne olacağını değiştirmek için değiştirici tuşlar ekleyin. Ayrıca, `Win` tuşunu basılı tutmadan tek seferlik bir sürükleme yakalamasını hazırlamak için tepsi simgesine sağ tıklayıp → **Yeni Kırpma** seçeneğini de kullanabilirsiniz.
- **Tablo:**

| Kısayol | Sonuç |
| --- | --- |
| `Win` + sürükle | Ekranda yüzen kırpma |
| `Win`+`Ctrl` + sürükle | Yüzen kırpma **ve** panoya kopyala |
| `Win`+`Alt` + sürükle | Yalnızca pano (galeriye eklenmez) |
| `Win`+`Shift` + sürükle | Yalnızca galeri (yüzen pencere yok) |
| `Win`+`Ctrl`+`Shift` + sürükle | Galeri **ve** pano, yüzen pencere yok |
| `Win`+`Shift`+`Space`, ardından sürükle | **Önce dondur** — ekranı dondurun, ardından sabit görüntü üzerinde seçim yapın |

- **CTA bağlantısı (metin):** **Tüm kısayolları ve sağ tık eylemlerini görün** → `DOCS`
- **Yerleşim notu:** `bg-soft` üzerinde tam genişlikte Markdown tablosu, tek aralıklı tuş çipleri; altında tek bir soluk belgeler bağlantısı.

---

## 5. Gizlilik ve açık kaynak

- **Bölüm adı:** `privacy`
- **Üst başlık:** Sizin, kendi makinenizde
- **Başlık:** Varsayılan olarak yerel. Lisans olarak açık.
- **Gövde metni:**
  > Yakalama ve OCR tamamen bilgisayarınızda gerçekleşir — bir kırpma almak veya dizinlemek için hiçbir yere hiçbir şey gönderilmez. Ağa dokunan tek özellikler bilerek tetiklediklerinizdir ve her biri verilerinizin tam olarak nereye gittiğini size söyler.
- **Güven kartları:**

| Kart başlığı | Tek satırlık açıklama |
| --- | --- |
| **Yakalama ve OCR tamamen yerel** | OCR, yerleşik `Windows.Media.Ocr` motorunu kullanır; bir kırpmayı yakalamak veya dizinlemek için makinenizden hiçbir şey ayrılmaz. |
| **Kayıt defteri olmadan oturum açılışında başlatma** | Snipdeck, bir `HKCU\…\Run` değeri yazmak yerine `shell:Startup` klasörünüze normal bir kısayol bırakır — sıradan, kullanıcı tarafından görülebilen otomatik başlatma mekanizması. |
| **Yüklemeler isteğe bağlıdır** | Önce hedefi adıyla belirten açık bir onay penceresi olmadan makinenizden hiçbir şey ayrılmaz. |
| **Çeviri yalnızca metin gönderir** | OCR + Çeviri, yalnızca tanınan *metni* Google'ın herkese açık uç noktasına gönderir — asla görüntüyü değil ve yalnızca siz istediğinizde. |
| **MIT lisanslı, kaynak GitHub'da** | Uygulamanın tamamı MIT lisansı altında açık kaynaktır. Okuyun, derleyin, çatallayın. |

- **Birincil CTA butonu:** **Kaynağı GitHub'da oku** → `GITHUB`
- **Yerleşim notu:** Güven kartı ızgarası artı belirgin bir GitHub butonu; Evreka güvenlik/özellik kartı stilini yeniden kullanın.

---

## 6. ReviseTouch tarafından yapıldı

- **Bölüm adı:** `who`
- **Üst başlık:** Snipdeck'in arkasında
- **Başlık:** ReviseTouch tarafından geliştirildi
- **Gövde metni:**
  > Snipdeck, **ReviseTouch**'tan ücretsiz, açık kaynaklı bir uygulamadır — geliştirici araçları ve yapay zekâ ürünleri geliştiriyoruz. Sitemizdeki her şeyin arkasındaki aynı ekip ve aynı standartlardır: yerel performans, dürüst gizlilik varsayılanları ve her gün kendimizin kullandığı araçlar.
- **CTA butonu:** **Başka neler geliştirdiğimize bakın** → `REVISETOUCH`
- **Yerleşim notu:** İnce bant (`bg-soft`, üst-alt kenarlık), ortalanmış kısa metin, revisetouch.com'a giden tek bir ghost buton.

---

## 7. Platformlar ve yol haritası

- **Bölüm adı:** `roadmap`
- **Üst başlık:** Nerede çalışır
- **Başlık:** Bugün Windows. macOS ve Linux yolda.
- **Gövde metni:**
  > Snipdeck şu anda yalnızca Windows içindir; çünkü yakalama, OCR, tepsi ve paylaşımın hepsi Windows'a özgü API'ler kullanır. Arayüz (Slint) ve çekirdeğin çoğu zaten platformlar arasıdır, dolayısıyla macOS ve Linux derlemeleri yolda.
- **Durum listesi:**

| Durum | Öğe |
| --- | --- |
| ✅ Mevcut | Windows 10 / 11 |
| 🚧 Yolda | macOS |
| 🚧 Yolda | Linux |
| 🚧 Planlanan | Uygulama içi ayarlar penceresi |
| 🚧 Planlanan | Daha fazla açıklama aracı (bulanıklaştırma / pikselleştirme, numaralı adım rozetleri, şekilleri taşıma/düzenleme) |

> **Not:** Ayarlar penceresi yayınlanana kadar yapılandırma `%APPDATA%\Snipdeck\settings.json` içinde bulunur — doğrudan düzenleyin, ardından Snipdeck'i yeniden başlatın. Belgeler her anahtarı kapsar.

- **CTA bağlantısı (metin):** **Yapılandırma referansı** → `DOCS`
- **Yerleşim notu:** İki sütun — bir tarafta yol haritası durum tablosu, diğerinde ayarlar notu vurgusu; Evreka teknik spesifikasyon tablosu çerçevelemesini yeniden kullanın.

---

## 8. Son indirme CTA'sı

- **Bölüm adı:** `download`
- **Üst başlık / rozet:** `ÜCRETSİZ · AÇIK KAYNAK · HESAP YOK`
- **Başlık:** Bir dakikadan kısa sürede kırpmaya başlayın.
- **Gövde metni:**
  > En güncel `snipdeck.exe` dosyasını Releases sayfasından alın — tek bir taşınabilir yürütülebilir dosya, kurulum yok. `Win` tuşunu basılı tutun, bir bölge sürükleyin, işte bu kadar.
- **Birincil CTA butonu:** **Windows için indir** → `DOWNLOAD`
- **İkincil CTA butonu:** **Belgeleri oku** → `DOCS`
- **Alt bilgi notu (küçük metin):**
  > Windows 10 / 11 · MIT lisanslı · Kendiniz derlemeyi mi tercih ediyorsunuz? `git clone` ve `cargo build --release`. [GitHub'da görüntüle](https://github.com/hasantr/snipdeck) → `GITHUB`
- **Yerleşim notu:** Radyal vurgu parıltısına sahip ters arka planlı CTA bölümü (Evreka `notify` deseni), ortalanmış başlık ve iki buton, altında soluk bir kaynaktan derleme notu.

---

## Ayrıca bkz.

- [Snipdeck belgeleri](/docs/snipdeck)
- [Yakalama kısayolları referansı](/docs/snipdeck)
- [ReviseTouch](https://revisetouch.com)
- [Snipdeck GitHub'da](https://github.com/hasantr/snipdeck)
