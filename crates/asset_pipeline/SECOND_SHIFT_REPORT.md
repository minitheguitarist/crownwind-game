# Prens piksel prototipi

Başlangıçta çalışma ağacı temizdi. İlk teslim `2698c39` commit’inde doğrulandı.

| Dosya | Teslim |
| --- | --- |
| `assets/characters/prince_debug.png` | Saydam 448×48 atlas. Soldan sağa 14 adet 32×48 kare. |
| `assets/previews/prince_debug.png` | Dört hareketi ayrı satırlarda gösteren 726×624 önizleme. |
| `content/characters/prince_debug.json` | idle 4, run 6, jump 2, fall 2 kare. Sabit pivot (16, 44). |
| `crates/asset_pipeline/tools/draw_prince.py` | Ortak yüz ve tunik çizimleri, pozlar, PNG üretimi ve yeniden üretim kontrolü. |
| `assets/ART_SPEC.md`, `assets/ASSET_PROVENANCE.md` | 14 renkli palet, çizim kararları, köken ve lisans kayıtları. |
| `crates/asset_pipeline/src/main.rs`, `src/manifest.rs`, `tests/cli.rs` (aynı crate), `content/README.md` | PNG başlığından kare ızgarasını ve indeks sınırlarını denetleyen isteğe bağlı CLI argümanı, testler ve kullanım. |

Açık mavi ve fildişi tunik koyu bordo pelerinden ayrışıyor. Kestane saçın dış tutamı ve pelerin ucu rüzgârla hareket ediyor. Koşuda öne çıkan bacak değişiyor. Zıplamada diz toplanıyor, düşüşte pelerin yukarı kalkıyor. Önizleme görsel olarak incelendi. Çizimler sağa bakıyor. Havada iki poz var. Oyun içinde hareket hızıyla adım uyumu henüz denenmedi.

## Doğrulama

- `cargo fmt --all -- --check` geçti.
- `cargo test --offline --locked -p asset_pipeline` geçti. 8 birim, 4 CLI testi.
- `cargo clippy --offline --locked -p asset_pipeline --all-targets -- -D warnings` uyarısız geçti.
- `python3 crates/asset_pipeline/tools/draw_prince.py --check` her iki PNG’yi yeniden üretilen baytlarla eşleştirdi. Manifestteki pivot, boyut ve kare dizileri de çizim kaynağıyla eşleşti.
- ImageMagick atlası 448×48, 14 görünür renk ve saydam arka plan olarak okudu. Rust denetimi PNG başlığı ve manifest geometrisini kapsar. Piksel verisini çözmez.

## Bevy entegrasyonu

1. Bevy varlık kökünden `characters/prince_debug.png` yükleyip en yakın komşu filtrelemesini kullanın. Atlas düzeni 14 sütun × 1 satır, hücre boyutu 32×48, boşluk ve kenar payı sıfır.
2. Hücrenin sol üstünden (16, 44) noktasını oyuncunun ayak konumuna oturtun. Merkezden çiziliyorsa sprite merkezini ayak noktasının 20 piksel üstüne yerleştirin. Sola bakarken yatay çevirin.
3. Manifestteki kare dizisini geçen süre × FPS ile ilerletin. idle ve run döngüye girer. jump ve fall son kareyi durum değişene kadar korur. Oyun durumunun ve çarpışma geometrisinin denetimi oynanış kodundadır.

Teslim dalı `agent/codex-assets`. Raporu içeren yerel commit’in SHA’sını öğrenmek için `git log -1 --format=%H -- crates/asset_pipeline/SECOND_SHIFT_REPORT.md` çalıştırılabilir.
