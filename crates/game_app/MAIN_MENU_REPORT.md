# Crownwind — Ana Menü ve Canlı Kamp Sahnesi (Rapor)

Dal: `integration/m3-pixellab-heroes` (görev metnindeki `integration/m2-visual-scale`
değil; çalışma ağacının gerçek ve en güncel dalı buydu, HEAD `ee99263`).
Motor: Bevy `=0.20.1` (yükseltilmedi). Commit, push, merge veya rebase yapılmadı.
Diğer worktree'lere dokunulmadı.

## 1. Eklenen ana menü sistemi

- **Durumlar:** `AppState { MainMenu, InGame }` (`menu/mod.rs`). Projede daha önce
  bir state yapısı yoktu. Oyun artık `MainMenu` ile açılıyor.
- **Dünya bir kez kuruluyor:** Eskiden `Startup`'ta çalışan `spawn_level`,
  `spawn_backdrop`, `spawn_camera` ve `spawn_hud` artık `OnEnter(InGame)` içinde
  çalışıyor. Tüm oyun `Update` zinciri `run_if(in_state(InGame))` ile korunuyor.
  Bu sayede menüdeyken fizik veya simülasyon çalışmıyor, zamanlayıcı ilerlemiyor
  ve hareket tuşları dünyayı etkilemiyor. Hareket, kamera, checkpoint, ölüm ve
  yeniden doğma kodlarına dokunulmadı.
- **Geçiş:** Tek Oyunculu'ya basılınca ekran 0,35 sn'de kararıyor, ardından
  `InGame` durumuna geçiliyor. Menü varlıkları (`DespawnOnExit`) siliniyor ve
  oyun dünyası kuruluyor. Mevcut hazırlık kapısı (`SceneReady`) geçince oyun
  0,4 sn'de beliriyor. Menü açıkken oyun görselleri arka planda önceden
  yükleniyor (`GamePreload`), bu yüzden geçişte takılma olmuyor.
- **Menü açılışı:** Menü görselleri GPU'ya ulaşana kadar ekran siyah kalıyor,
  sonra 0,5 sn'de beliriyor. İlk karede siyah veya yarım sahne görünmüyor.
- **Görüntü ölçeği:** Menü kamerası, oyunla aynı kuralı kullanıyor: 640×360
  sanal çözünürlük, tam sayı ölçek ve letterbox (`view::integer_scale`,
  `view::letterbox`). UI, 1 UI birimi = 1 render pikseli olacak şekilde menü
  kamerasının viewport'una bağlı. Bu sayede logo ve yazılar sahneyle aynı
  piksel ızgarasında duruyor ve bulanıklaşmıyor.

### Menü butonlarının durumları

| Sıra | Buton | Durum | Davranış |
| --- | --- | --- | --- |
| 1 | Tek Oyunculu | **Aktif** | Fare tıklaması, Enter veya Space ile oyun başlıyor. |
| 2 | Çok Oyunculu | **Pasif** | Soluk görünüyor, `YAKINDA` etiketi var. Tıklama ve Enter hiçbir şey yapmıyor. Seçilince altta "Çok oyunculu mod yakında geliyor" yazıyor. |
| 3 | Ayarlar | **Pasif** | Aynı görünüm. Seçilince "Ayarlar henüz hazır değil" yazıyor. |

- Butonlar gerçek Bevy UI düğümleri. Görünümleri yarı saydam gece mavisi cam,
  1 px eskitilmiş altın kenarlık ve üst kenarda ince bir cam parlaması.
  Seçili butonun solunda altın bir elmas işaret ve daha parlak bir çerçeve var.
- Fare üzerine gelince buton seçiliyor. Yön tuşları veya W/S ile gezinme var
  (döngüsel). Enter, NumpadEnter veya Space seçili butonu çalıştırıyor, ama
  yalnızca aktif olanı. Pasif butonlar da seçilebiliyor; böylece neden
  kullanılamadıklarını açıklayan yazı okunabiliyor, fakat çalıştırılamıyorlar.
- Etkinleştirme kuralı tek yerde: `MenuItem::enabled()` ve
  `MenuModel::activate()`. İleride Ayarlar veya Çok Oyunculu'yu açmak için
  burada değişiklik yapıp ilgili `MenuAction`'ı eklemek yetecek.
- Yazılar arka plan görseline gömülü değil. Etiketler çalışma anında koddan bir
  bitmap fonta rasterize ediliyor (`pixel_font.rs`). Türkçe karakterlerin
  (Ç, ı, ş, ğ, ö, ü, İ) hepsi mevcut ve kenar yumuşatma yok.

## 2. Yeni dosyalar ve değiştirilen kod

Yeni (`crates/game_bevy/src/menu/`):
- `mod.rs`: `AppState`, `MenuPlugin`, menü kamerası ve ölçek, geçiş ve
  kararmalar, oyun görsellerinin önceden yüklenmesi, tekil dünya kontrolü,
  geliştirici ortam değişkenleri.
- `model.rs`: Motordan bağımsız kurallar (buton sırası ve durumları, gezinme,
  etkinleştirme), zamana bağlı kare seçimi, düzensiz göz kırpma zamanlayıcısı.
- `scene.rs`: Kamp sahnesinin katmanları ve tüm animasyon sistemleri.
- `ui.rs`: Logo, butonlar, ipucu satırları, kararma katmanı, fare ve klavye
  etkileşimi.
- `pixel_font.rs`: Bitmap font rasterizer.
- `camp_layout.rs` ve `font_data.rs`: Betik tarafından üretiliyor, elle
  düzenlenmemeli.

Yeni araç: `crates/game_bevy/tools/menu_art/` (`build_menu_art.py`, `README.md`,
`camp_source.webp`).

Değişen:
- `crates/game_bevy/src/lib.rs`: State kaydı, oyun kurulumunun
  `OnEnter(InGame)`'e taşınması ve `Update` zincirinin state ile korunması.
  Review kurulumu da `OnEnter(InGame)`'e alındı.
- `crates/game_bevy/src/debug.rs`: Menüden başlayan çalıştırmalarda ekran
  görüntüsü zamanlayıcısı menü hazır olunca başlıyor. Ayrıca
  `CROWNWIND_FPS_LOG=1` eklendi.

Mevcut oyun sistemleri yeniden düzenlenmedi. Daha önce üretilip henüz entegre
edilmemiş karakter animasyonlarına dokunulmadı.

## 3. Kullanılan görsel varlıklar

Kaynak, görevle birlikte paylaşılan 1672×941 boyutundaki kamp illüstrasyonu.
İçinde prens ve prenses ateş başında yan yana oturuyor ve sol taraf boş.
Karakterler onaylı tasarımlarla örtüşüyor: prenste kestane saç, gece mavisi
kıyafet, altın işleme, metal eldivenler ve bordo pelerin; prenseste altın
sarısı uzun saç, turkuaz gözler, pembe çiçek, turkuaz kurdele, pembe/krem
elbise ve turkuaz takılar. Bu nedenle **yeni karakter çizilmedi**, yeni oturma
pozu da üretilmedi. Çerçevelenmiş görüntü 640×360'a box-filter ile indirildi.
Kaynağın doğal piksel ızgarası yaklaşık 2,6 px olduğu için yüzler ve oranlar
korunuyor. Kontrol için onaylı anchor'lara bakıldı:
`crownwind-art-review/claude/exports/{prince,princess}/CROWN_*_ANCHOR_*.png` ve
`input/approved/princess_sheet_approved.webp`.

Varlıklar `assets/menu/` altında, toplam yaklaşık 8 MB:
- `camp_base.png`: Sabit 640×360 plaka.
- `camp_canopy_left.png`, `camp_canopy_right.png` (yapraklar ve fener),
  `camp_grass.png`, `camp_prince.png`, `camp_princess.png`, `camp_fire.png`:
  Animasyon atlasları. Her biri yalnızca değişen bölgeyi içeriyor.
- `blink_prince.png`, `blink_princess.png`: Yarı kapalı ve kapalı göz kapağı.
- `logo.png`: Kodla yerleştirilen CROWNWIND yazısı. Altın gradyan, koyu kontur,
  taç ve alt çizgi var.
- `fire_glow.png`, `menu_shade.png` (sol okunabilirlik gölgesi), `mist.png`.

Bütün dosyaların diskte olduğu bir testle doğrulanıyor:
`menu_assets_exist_on_disk`.

**Kaynak/lisans kaydı:** `assets/ASSET_PROVENANCE.md` Codex'e ait bir dosya
olduğu için bu satırları oraya ekleme işlemi engellendi; dosyaya dokunulmadı.
Kaynak bilgisi `crates/game_bevy/tools/menu_art/README.md` içinde duruyor.
Entegratörün bu satırları `ASSET_PROVENANCE.md`'ye eklemesi önerilir.
`assets/menu/` klasörünü, görevdeki açık talimat üzerine ben ekledim. Normalde
`assets/**` Codex'in sahiplik alanında; bunu bilerek yaptım.

## 4. PixelLab ve Aseprite MCP kullanımı

- **PixelLab:** Kullanılmadı ve **kredi harcanmadı**. Onaylı karakterler,
  paylaşılan illüstrasyonda zaten oturur pozda bulunuyordu. Göz kırpma ve
  rüzgâr da mevcut piksellerden üretilebildiği için yeni üretime gerek kalmadı.
- **Aseprite MCP:** Kullanılmadı. Kare üretimi, maskeler ve atlas paketleme
  tekrarlanabilir olsun diye Python, Pillow ve NumPy betiğiyle yapıldı
  (`build_menu_art.py`). GIF'ler ffmpeg ile oluşturuldu.

## 5. Kamp sahnesi animasyonları

Hepsi duvar saatine (`Time<Real>`) bağlı, oyunun FPS'inden bağımsız
(`loop_frame` testi). Rüzgâr ve nefes 5 sn'lik döngüde (40 kare, 8 fps), ateş
1,33 sn'lik döngüde (16 kare, 12 fps). Bütün hareketler döngünün tam sayı
frekanslarından üretildiği için atlaslar dikişsiz dönüyor.

- **Prens:** Saç uçları hafifçe kıpırdıyor. Bordo pelerin rüzgârda
  dalgalanıyor; omuza yakın kısım sabit kalıyor, uca doğru hareket artıyor.
  Göğüs ve omuzlarda yavaş nefes hareketi var. Düzensiz aralıklarla göz
  kırpıyor (yarı kapalı → kapalı → yarı kapalı, toplam 0,19 sn) ve ara sıra
  çift kırpma yapıyor.
- **Prenses:** Uzun saç uca doğru artan bir dalgayla hareket ediyor. Kurdele,
  sol saç tutamları ve elbise eteği hafifçe dalgalanıyor. Nefes hareketi
  prensinkinden faz kaydırılmış. Kendi tohumuyla bağımsız göz kırpıyor.
  Yakalamada iki karakterin göz kırpması hiç çakışmadı; testte de aynı anda
  kapalı kalma oranı %10'un altında.
- **Çevre:** Ateşteki alevler yükselip titreşiyor ve parlaklıkları değişiyor.
  9 kıvılcım yükselip sönüyor, 6 hafif duman pikseli rüzgârla sola kayıyor.
  Ateş ışığı 4 kademeli olarak titriyor. Üst kanopi yaprakları ve lupinler
  sallanıyor, çimen ve beyaz çiçekler sol altta salınıyor, fener hafifçe
  sallanıyor. Sol taraftaki uzak ormanın üzerinde iki soluk sis bandı ileri
  geri kayıyor ve derinlik hissi veriyor. 4 yaprak ekranın üstünde süzülüyor.
- **Göz ve yüz koruması:** Göz bölgeleri tüm rüzgâr ve nefes alanlarından
  maskelendi. Yüzde hiçbir piksel kaymıyor; yalnızca göz kapağı katmanları
  değişiyor.
- **Kumlanma düzeltmesi:** İlk sürümde her karede yalnızca değişen pikseller
  yazılıyordu ve pelerin kenarında karıncalanma görünüyordu. Artık her kare,
  bütün karelerde değişen alanların birleşimini tamamen kapsıyor.

## 6. Tek Oyunculu geçişi

Çalışıyor. Otomatik tıklama, gerçek tıklama ile aynı kod yolundan geçiyor:
```
CROWNWIND_MENU=1 CROWNWIND_MENU_AUTOSTART=1.0 CROWNWIND_SCREENSHOT=… ./target/release/game_app
menu: single player selected
artwork loaded
scene ready …
world check: 1 game camera, 0 menu cameras, 1 player
```
Geçişten sonra prens kontrol edilebiliyor; ekran görüntüsünde HUD sayacı
işliyor. Kamera, oyuncu ve dünya iki kez oluşturulmuyor; her girişte bu bir
log satırıyla kontrol ediliyor. Mevcut autoplay rotası menü atlanarak
çalıştırıldığında önceki sonucu birebir veriyor: **0:30.88, 0 ölüm, 5/5
checkpoint**. Review sandbox'ı da çalışıyor.

## 7. Derleme ve test sonuçları

- `cargo fmt --all -- --check`: geçti
- `cargo clippy --workspace --all-targets -- -D warnings`: geçti
- `cargo test --workspace`: geçti. game_bevy 64, game_core 97 (1 ignored,
  daha önceden var), asset_pipeline 8 + 4. Yeni testler: buton sırası ve
  durumları, pasif butonların hiçbir yoldan çalışmaması, klavyeyle pasif
  butonun Enter/Space ile tetiklenmemesi (Bevy `App` üzerinde gerçek
  `ButtonInput` ile), klavyeyle Tek Oyunculu'nun başlaması, döngüsel gezinme,
  FPS'ten bağımsız kare seçimi, göz kırpmanın düzensiz ve senkronsuz olması,
  yazıların kenar yumuşatmasız olması ve tüm glifleri içermesi, patch
  kenarlarının tam piksele oturması, parçacıkların sahne içinde kalması,
  varlıkların diskte bulunması, menü açılış kararması.
- `cargo build --release -p game_app`: geçti
- Performans (1280×720, `CROWNWIND_FPS_LOG=1`, bu makinede): release'te menü yaklaşık
  **200 FPS**. Debug derlemede menü yaklaşık 71 FPS, oyun yaklaşık 55 FPS;
  yani menü oyundan daha hafif.

## 8. Ekran görüntüleri ve önizlemeler

`crates/game_app/screenshots/menu/`:
- `main_menu.png`: Ana menü (1280×720, 2× ölçek).
- `after_single_player.png`: Tek Oyunculu'dan sonra oyun.
- `camp_menu_preview.gif`: Gerçek oyundan yakalanan 8 sn, 12 fps, 640×360.
- `camp_characters_closeup.gif`: Karakterler ve ateş yakın planı. Rüzgâr,
  nefes ve göz kırpma burada daha iyi görünüyor.
- `autoplay_finish_check.png`, `review_check.png`: Mevcut akışların
  regresyon kontrolü.

Görsel kalite, yakalanan kareler tek tek incelenerek değerlendirildi. Her kare
geçişinde sahnede değişen pikseller var (en az 224, ortalama yaklaşık 13 bin
fiziksel piksel), yani animasyon gerçekten oynuyor. Göz kırpma kareleri
yakalamada tespit edildi: prens 4–5, 9–10 (çift kırpma) ve 51–52. karelerde,
prenses 23–25 ve 78–80. karelerde. Yüzler ve oranlar bozulmuyor.

## 9. Geliştirici anahtarları

- `CROWNWIND_SKIP_MENU=1`: Doğrudan oyuna girer. `CROWNWIND_AUTOPLAY`,
  `CROWNWIND_REVIEW` ve ekran görüntüsü çalıştırmaları eski betikler bozulmasın
  diye menüyü zaten atlıyor.
- `CROWNWIND_MENU=1`: Ekran görüntüsü çalıştırmalarında menüyü zorlar.
- `CROWNWIND_MENU_AUTOSTART=<sn>`: Tek Oyunculu'yu otomatik başlatır.
- `CROWNWIND_MENU_FRAMES=<klasör>` (+ `_STEP`, `_SECS`): GIF için kare yakalar.
- `CROWNWIND_FPS_LOG=1`: FPS günlüğü.

## 10. Bilinen eksikler

- Fareyle pasif butona tıklamanın hiçbir şey yapmadığı aynı model fonksiyonuyla
  (`activate`) güvenceye alındı ve birim testle doğrulandı. Ancak gerçek bir
  fare tıklaması otomatik olarak simüle edilmedi. Klavye yolu Bevy düzeyinde
  test edildi. Pasif butonun seçili/hover durumu için ayrı bir ekran görüntüsü
  alınmadı.
- Farklı pencere boyutları ekran görüntüsüyle değil, mevcut letterbox ve ölçek
  testleriyle doğrulandı. 640×360'tan küçük pencerede menü tam sayı olmayan
  ölçeğe düşüyor; bu durumda yazılar hafif yumuşayabilir.
- Animasyonlar tek bir illüstrasyonun piksellerini bükerek üretildi. Hareketler
  bilerek çok küçük (en fazla yaklaşık 1–2 px) tutuldu; daha büyük hareketler
  ayrı çizilmiş katmanlar gerektirir.
- Uzak orman ve dağlar düz plakanın parçası, gerçek parallax yok. Derinlik
  hissi sis bantlarıyla sağlanıyor.
- MP4 üretilemedi (sistem ffmpeg'inde libx264 yok); önizlemeler GIF.
- Oyun içinden menüye dönüş yok; görevde istenmiyordu.
- `ASSET_PROVENANCE.md` güncellenemedi (Codex dosyası, bkz. §3).
- Değişiklikler commit edilmedi.
