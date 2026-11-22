# Codex FIRST SHIFT raporu

Çalışma dizini `/home/smurat/Projects/crownwind-codex`, dal `agent/codex-assets`. Başlangıçta çalışma ağacı temizdi.

## Teslim

| Değişen dosya | İçerik |
| --- | --- |
| `crates/asset_pipeline/src/main.rs` | İçerik dizinlerini veya belirtilen tek dosyayı doğrulayan CLI. Hatalarda dosya yolunu ve nedeni bildirir. |
| `crates/asset_pipeline/src/manifest.rs` | Sabit v1 şemalarının alan, tür ve aralık denetimleri. Klip içinde yinelenen indeksleri, sınır dışındaki geometriyi ve katı karoda doğmayı yakalar. Altı birim testi içerir. |
| `crates/asset_pipeline/tests/cli.rs` | İki örneği, toplu taramayı, tanıları ve çıkış kodlarını sınayan üç entegrasyon testi. |
| `content/README.md` | Kullanım, sayı gösterimi ve karo sınırları. |
| `content/characters/prince_debug.json` | Dört hareket durumu ve 13 hücre için özgün geçici animasyon planı. |
| `content/levels/wind_steps.json` | 16 piksel karolarla 30×12 boyutunda özgün deneme parkuru. |
| `assets/ART_SPEC.md` | Palet, silüet, pivot, saç ve pelerin hareketi, kaynak ve lisans kaydı. |
| `docs/reports/codex-first-shift.md` | Bu teslim raporu. |

## Kontroller

| Komut | Sonuç |
| --- | --- |
| `cargo fmt --all -- --check` | Geçti. |
| `cargo test --offline --locked -p asset_pipeline` | 6 birim ve 3 CLI testi geçti. |
| `cargo clippy --offline --locked -p asset_pipeline --all-targets -- -D warnings` | İlk turdaki gereksiz kapanış düzeltildi. Son tur uyarısız geçti. |

Kapsam tek karoluk doğma noktasını denetler. Oyuncu hitbox boyutu ve parkurun oynanabilirliği daha sonraki oyun entegrasyonunda sınanacak. Animasyon görselleri henüz çizilmedi. Tamsayı alanları u64 aralığını, ondalık ve üslü gösterimler serde_json kayan nokta hassasiyetini kullanır.

## Yerel commit ve entegrasyon

Commit SHA’sı yok. Dosyalara yazma başarılı oldu. `git add`, bağlı çalışma ağacının indeks kilidini oluştururken `Read-only file system` hatası verdi. Kilit yolu `/home/smurat/Projects/crownwind-game/.git/worktrees/crownwind-codex/index.lock`. FIRST SHIFT yetki yükseltmeye izin vermiyor.

Git çalışma ağacı üstverisine yazılabilen bir oturumda, bu çalışma dizininde ve `agent/codex-assets` dalında yerel commit için:

```sh
git add -- assets/ART_SPEC.md content/README.md content/characters/prince_debug.json content/levels/wind_steps.json crates/asset_pipeline/src/main.rs crates/asset_pipeline/src/manifest.rs crates/asset_pipeline/tests/cli.rs docs/reports/codex-first-shift.md
git diff --cached --check
git commit -m "Validate Crownwind animation and level manifests"
```

Entegratör yerel commit'i inceledikten sonra entegrasyon dalında çalışma alanının testlerini çalıştırabilir. Oyun yükleyicisi bağlanırken seviye örneğinin sol üst başlangıç ve aşağı doğru artan y ekseni kuralı doğrulanmalı.
