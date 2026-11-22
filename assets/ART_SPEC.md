# First movement sprite study

Use the 480×270 reference viewport and 16×16 tiles in Wind Steps. Draw the debug
prince facing right in a 32×48 cell. Measure from its upper-left corner and keep
the planted boot at the (16, 44) pivot across frames. Preview at native scale
and 2× nearest-neighbor. Keep contour changes on whole pixels.

Start with ink plum (#292334), slate (#53516B), warm clay (#B77257), pale amber
(#EBC995), and a teal cape (#438781). Reserve the lightest shade for the face
and readable edges. In a flat silhouette the prince should read from his
medium-length hair, forward boot and long split cape. Leave a gap between cape
and rear heel on the run contact poses.

| State | Cells | Drawing plan |
| --- | --- | --- |
| idle | 0–3 | Keep boots and shoulders steady. Let a breeze lift the hair tip, then the cape hem one frame later. Ease the hem back toward cell 0. |
| run | 4–9 | Alternate contact, compression and passing poses for each leg. Keep the cape attachment at the collar and delay the hem at each reversal. |
| jump | 10–11 | Draw push-off and rising poses. Let the hair and hem trail below the collar. |
| fall | 12 | Lift the hem above its resting line and separate the boots. |

Use lowercase snake_case state names for new clips. Set playback speed with the
manifest FPS and elapsed time. Check the idle and run loop seams at their authored
speeds. Overlay neighboring frames at the pivot to spot palette drift, hair length
changes and displaced feet, especially when evaluating AI-generated candidates.

This sheet plan and palette are original placeholders. For each imported or
generated image, record the path, creator/tool, date, source and permission or
license in `assets/ASSET_PROVENANCE.md` before inclusion. Keep attribution text
where the license requires it. This shift supplies data only.
