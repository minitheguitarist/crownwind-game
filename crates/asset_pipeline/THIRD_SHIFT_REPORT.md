# Windwood Forest teslimi

| Dosya | Boyut ve içerik |
| --- | --- |
| `assets/environment/windwood/tiles.png` | 176×16 RGBA. 11 adet 16×16 karo. Çimen, iki toprak çeşidi, taş, yosunlu taş, iki kenar, eğrelti, kök, çiçek ve diken. |
| `assets/environment/windwood/sky.png` | 480×270. Opak gökyüzü, dağlar ve katmanlı pus. |
| `assets/environment/windwood/forest.png` | 480×270. Saydam gökyüzü boşlukları olan kozalaklı orman silüeti. |
| `assets/previews/windwood.png` | 480×270. Prensin 32×48 çizimini 1× ölçekte kullanan platform kompozisyonu. |

`crates/asset_pipeline/tools/draw_windwood.py` dört PNG’yi üretir. Karo
koordinatları, palet ve yeniden üretim komutları
`assets/environment/windwood/README.md` içinde. Dört görselin lisans kaydı
`assets/ASSET_PROVENANCE.md` dosyasına eklendi.

- `cargo fmt --all -- --check` geçti.
- `cargo test --offline --locked -p asset_pipeline` geçti. 8 birim ve 4 CLI testi.
- `cargo clippy --offline --locked -p asset_pipeline --all-targets -- -D warnings` geçti.
- `python3 -B crates/asset_pipeline/tools/draw_windwood.py --check` karo geometrisini, kesik karolardaki saydamlığı ve dört dosyanın yeniden üretimini doğruladı. ImageMagick boyutları ve opaklık durumlarını doğruladı.
- `python3 -B crates/asset_pipeline/tools/draw_prince.py --check` mevcut prens görsellerini doğruladı.

Bevy’de 11×1 atlas ızgarası ve en yakın komşu örnekleme kullanın. Gökyüzü
orman katmanından daha az kaymalı. Arka planlar birer ekran genişliğinde ve
kenarları tekrar dikişi için çizilmedi. Kamera kaydırması için bu katmanları
genişletin veya görünür kaymayı sınırlandırın. Diken ve zemin çarpışmalarını
seviye verisinde tanımlayın. Önizleme sabit bir sahne. Atlama mesafeleri ve
tehlike sınırları oyun içinde denenmedi.

Yerel teslim commit’ini `git log -1 --format=%H -- crates/asset_pipeline/THIRD_SHIFT_REPORT.md`
ile bulun.
