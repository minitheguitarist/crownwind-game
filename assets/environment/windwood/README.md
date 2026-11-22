# Windwood Forest

The RGBA atlas is 176×16. Its eleven 16×16 tiles are packed left to right without padding. Coordinates start at the upper left.

| Index | x | Tile |
| --- | --- | --- |
| 0 | 0 | Grass over soil |
| 1 | 16 | Earth, dark clusters |
| 2 | 32 | Earth, alternate clusters |
| 3 | 48 | Jointed stone |
| 4 | 64 | Moss over stone |
| 5 | 80 | Left grass edge |
| 6 | 96 | Right grass edge |
| 7 | 112 | Fern |
| 8 | 128 | Small exposed root |
| 9 | 144 | Forest flowers |
| 10 | 160 | Sharp burgundy thorns |

The foreground shares the prince's ink (#292334), leather shadow (#573435)
and slate (#48536B). Olive moss (#718151) marks walkable surfaces. Rose
thorn faces (#974862) and pale tips (#F2C8A0) make hazards easy to pick out.
Edges, plants, roots and thorns have transparent cutouts. Earth and stone
fill their cells for continuous surfaces.

Both backdrop plates measure 480×270. The opaque sky plate uses stepped haze,
mountain faces and a warm sun. The transparent forest plate uses two bands of
cool conifers. Their lower contrast keeps the prince readable. The visual
composition in `assets/previews/windwood.png` uses atlas tiles and the shared
drawing master for prince idle frame 2 at native scale.

## Regeneration

Run from the workspace root using Python 3 and its standard library.

```sh
python3 -B crates/asset_pipeline/tools/draw_windwood.py
python3 -B crates/asset_pipeline/tools/draw_windwood.py --check
```

The check verifies cell dimensions and transparency for the cutout tiles,
then compares all four generated PNGs byte for byte.

## Bevy integration

Use nearest-neighbor sampling, integer presentation scaling and an 11×1
16×16 atlas grid. Draw sky behind forest, then terrain, ornaments and the
player. Prototype parallax camera factors of 0.15 for sky and 0.4 for forest
are starting values. Both plates are single views. Extend the artwork or
bound layer scrolling to keep texture edges out of view. Assign collision
and hazard behavior independently through level data.
