# Main-menu camp scene: art build

`build_menu_art.py` turns `camp_source.webp` (the user-supplied 1672×941
camp illustration of the approved prince and princess) into the runtime
assets under `assets/menu/` and two generated Rust tables:

- `crates/game_bevy/src/menu/camp_layout.rs` – patch positions, frame counts, fps
- `crates/game_bevy/src/menu/font_data.rs` – 1-bit bitmap glyphs for menu labels

```bash
python3 crates/game_bevy/tools/menu_art/build_menu_art.py
```

Needs Python 3 with Pillow and NumPy, and the DejaVu fonts
(`/usr/share/fonts/dejavu-*`). Run `cargo fmt` afterwards.

## How the animation is made

The source is box-filtered to the 640×360 render resolution (its native
pixel grid is ~2.6 source px). Wind, breathing and fire flicker are smooth
displacement fields applied to the full-resolution source inside hand-placed
region masks (hair, cape, ribbon, dress hem, lupines, canopy leaves, grass,
lantern, flames), then box-filtered the same way. Each patch atlas keeps
only the area that ever changes; within that area every frame is opaque, so
moved and unmoved pixels never mix. Eyes are masked out of every field; the
blinks are separate hand-placed eyelid overlays (half-closed, closed).

All motions use whole-number frequencies of their loop (5 s for wind and
breathing, 1.33 s for the fire), so every atlas loops seamlessly.

## Licenses

- `camp_source.webp`: Crownwind project art supplied by the project owner.
- Bitmap glyphs in `font_data.rs` are rasterized from DejaVu Serif Bold and
  DejaVu Sans Bold. DejaVu fonts are under the Bitstream Vera license plus
  public-domain changes, which permit derived works provided the result is
  not named "Bitstream" or "Vera"; the glyph table is a derived bitmap, not a
  redistributed font file.
