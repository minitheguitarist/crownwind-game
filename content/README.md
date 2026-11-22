# Content

Data-only content owned by Codex during parallel work. JSON schemas in `docs/contracts/` are frozen on the initial branches.

From the workspace root, run `cargo run --locked -p asset_pipeline` to validate every
direct `*.json` entry in `content/characters/` and `content/levels/`. To check one file:

```sh
cargo run --locked -p asset_pipeline -- animation content/characters/prince_debug.json
cargo run --locked -p asset_pipeline -- level content/levels/wind_steps.json
```

The CLI exits silently with 0 on success, prints the first file and issue to stderr
with exit code 1 on invalid or unreadable input, and uses 2 for incorrect arguments.
The Rust checks implement the two frozen v1 contracts explicitly. Integer fields
fit in an unsigned 64-bit value. Decimal and exponent forms use serde_json's
floating-point precision. Use integer literals for exact large indices.

The original `prince_debug` placeholder describes planned atlas cells. Indices
must be unique within each clip and may be shared between clips. Atlas artwork
is pending. The original `wind_steps` placeholder uses tile coordinates with
an upper-left origin and increasing y downward. Each solid includes its starting
tile and excludes its ending coordinates (x+w, y+h). Every solid and the spawn
tile must fit within the level, and the spawn tile must be clear.
