# Game Design Draft v0.1

## Elevator pitch
Original high-detail pixel-art 2D fantasy platform adventure. The story begins when the princess is abducted and a young prince sets out to find her. Gameplay starts as movement/parkour before the first equipment decision, and grows into exploration, action combat, and progression.

## Characters
- Prince: medium-length hair, expressive readable silhouette, long flowing cape, visually reacts to wind; platforming first.
- Princess: long layered golden-blonde hair, carefully designed face and outfit, subtle idle hair flutter; active role in story via clues and cutscenes.
- All characters original. Initial playable character can be a colored rectangular debug placeholder. No final art commitments yet.

## Later progression (NOT in Milestone 0)
- Four optional starting combat paths: staff mage, magic gauntlets, sword/shield, dual daggers.
- Shared traversal progression separate from combat equipment.
- Stats under consideration: health, stamina, strength, magic, agility.
- One connected, non-trivial world made of regions; no bloated content requirements for first prototype.

## Cutscenes (later)
Storybook still images, gentle zoom/pan, limited parallax. A simple data-driven cinematic player can be built after the first movement slice.

## Pixel art target
- Virtual viewport prototype: 480×270 with nearest-neighbor rendering; desktop window initially 1280×720 or 960×540 depending integer-scaling strategy, don't stretch non-integer pixel grids.
- Initial tile unit: 16×16 or 32×32; choose explicitly within level prototype, do not mix accidentally.
- Animations designed around readable silhouettes, timing, anticipation, follow-through; 2–3 hair frames alone do not guarantee fluidity.
- Pixel-art wind animation can use hand-drawn frames or carefully quantized secondary motion; avoid subpixel shimmering at pixel scale.

## Initial acceptance criteria
1. Linux/Fedora desktop app starts with a visible debug environment and player.
2. Left/right movement, jump, coyote time, buffered jump, horizontal collision and landing are functional.
3. Camera tracks the player without disorienting jitter.
4. A short procedural obstacle course is traversable; reset after falling.
5. Core tests pass; deterministic updates don't depend on render FPS.
6. No unlicensed textures or music added.
