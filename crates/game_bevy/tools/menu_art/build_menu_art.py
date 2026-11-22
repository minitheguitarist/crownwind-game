#!/usr/bin/env python3
"""Build the Crownwind main-menu camp scene assets.

Source: `camp_source.webp` (user-supplied 1672x941 pixel-art illustration of
the prince and princess at the Windwood camp). Everything is derived from it
at the game's 640x360 render resolution:

- `assets/menu/camp_base.png`      static 640x360 plate
- `assets/menu/camp_<group>.png`    animated patch atlases (only the pixels
                                    that change; the rest is transparent)
- `assets/menu/blink_<who>.png`     half / closed eyelid overlays
- `assets/menu/fire_glow.png`, `menu_shade.png`, `mist.png`, `logo.png`
- `crates/game_bevy/src/menu/camp_layout.rs`   generated placement table
- `crates/game_bevy/src/menu/font_data.rs`     generated bitmap glyphs

Wind / breathing are made by a smooth displacement field applied to the
full-resolution source inside hand-placed region masks, then box-filtered to
640x360 exactly like the base plate, so static pixels never shimmer. Every
motion uses whole-number frequencies of the loop, so each atlas loops
seamlessly.

Run from the workspace root:  python3 crates/game_bevy/tools/menu_art/build_menu_art.py
Needs Pillow + NumPy, and the DejaVu fonts (system package) for the glyphs.
"""

from __future__ import annotations

import math
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
OUT = os.path.join(ROOT, "assets", "menu")
RS_DIR = os.path.join(ROOT, "crates", "game_bevy", "src", "menu")

W, H = 640, 360
TAU = 2.0 * math.pi

SRC_IMG = Image.open(os.path.join(HERE, "camp_source.webp")).convert("RGB")
SRC = np.asarray(SRC_IMG, dtype=np.float32)
SH, SW = SRC.shape[:2]
FX, FY = SW / W, SH / H


def down(arr: np.ndarray) -> np.ndarray:
    img = Image.fromarray(np.clip(arr + 0.5, 0, 255).astype(np.uint8))
    return np.asarray(img.resize((W, H), Image.Resampling.BOX), dtype=np.uint8)


BASE = down(SRC)

# Source-resolution pixel coordinates expressed in 640-space.
_YY, _XX = np.mgrid[0:SH, 0:SW].astype(np.float32)
X640 = (_XX + 0.5) / FX
Y640 = (_YY + 0.5) / FY


def poly_mask(points, feather=3.0) -> np.ndarray:
    m = Image.new("L", (SW, SH), 0)
    ImageDraw.Draw(m).polygon([(x * FX, y * FY) for x, y in points], fill=255)
    if feather > 0:
        m = m.filter(ImageFilter.GaussianBlur(feather * FX * 0.5))
    return np.asarray(m, dtype=np.float32) / 255.0


def box_mask(x0, y0, x1, y1, feather=1.0):
    return poly_mask([(x0, y0), (x1, y0), (x1, y1), (x0, y1)], feather)


def soften(mask: np.ndarray, radius640: float) -> np.ndarray:
    m = Image.fromarray(np.clip(mask * 255, 0, 255).astype(np.uint8))
    m = m.filter(ImageFilter.MaxFilter(3)).filter(ImageFilter.GaussianBlur(radius640 * FX))
    return np.asarray(m, dtype=np.float32) / 255.0


R, G, B = SRC[..., 0], SRC[..., 1], SRC[..., 2]
LUM = 0.3 * R + 0.59 * G + 0.11 * B
LEAF = ((G > 70) & (G >= B) & (R > 50) & (G > 0.7 * R)).astype(np.float32)
GRASS = (((G > 45) & (G > B * 0.95) & (G > R * 0.9)) | ((LUM > 150) & (np.abs(R - B) < 60))).astype(
    np.float32
)
PURPLE = ((B > R * 0.9) & (B > G * 1.15) & (B > 60)).astype(np.float32)
FLAME = ((R > 170) & (G > 80) & (R - B > 70)).astype(np.float32)


def clip01(v):
    return np.clip(v, 0.0, 1.0)


def wind(t, x, speed=1.0):
    """Gentle gust in [-1, 1]; `t` is the loop phase in [0, 1)."""
    return 0.65 * np.sin(TAU * (t + x / 180.0)) + 0.35 * np.sin(TAU * (2 * t + x / 90.0) + 1.3)


def breath(t):
    return 0.5 - 0.5 * math.cos(TAU * t)


def warp(src: np.ndarray, dx: np.ndarray, dy: np.ndarray) -> np.ndarray:
    """Inverse bilinear warp: out(x, y) = src(x - dx, y - dy) (source px)."""
    h, w = dx.shape
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    sx = np.clip(xx - dx, 0, w - 1.001)
    sy = np.clip(yy - dy, 0, h - 1.001)
    x0 = np.floor(sx).astype(np.int32)
    y0 = np.floor(sy).astype(np.int32)
    fx = (sx - x0)[..., None]
    fy = (sy - y0)[..., None]
    a = src[y0, x0]
    b = src[y0, x0 + 1]
    c = src[y0 + 1, x0]
    d = src[y0 + 1, x0 + 1]
    return (a * (1 - fx) + b * fx) * (1 - fy) + (c * (1 - fx) + d * fx) * fy


# --------------------------------------------------------------------------
# Animated groups
# --------------------------------------------------------------------------

LOOP_FRAMES = 40  # 5 s at 8 fps
LOOP_FPS = 8.0

PRINCE_EYES = (437, 98, 460, 112)
PRINCESS_EYES = (497, 111, 521, 127)


def protect(mask, *boxes):
    for b in boxes:
        mask = mask * (1.0 - box_mask(*b, feather=2.0))
    return mask


def prince_field(t):
    hair = protect(
        poly_mask(
            [(404, 74), (468, 74), (469, 90), (452, 93), (438, 95), (430, 100),
             (427, 128), (402, 128)],
            3,
        ),
        PRINCE_EYES,
    )
    lever_c = clip01((365.0 - X640) / 130.0) ** 1.3
    cape = poly_mask(
        [(210, 192), (238, 160), (262, 141), (300, 138), (336, 143), (362, 150),
         (352, 176), (330, 196), (300, 216), (270, 242), (210, 248)],
        4,
    )
    chest = protect(
        poly_mask([(384, 126), (502, 126), (504, 206), (382, 206)], 10)
        * clip01((Y640 - 118.0) / 10.0),
        PRINCE_EYES,
    )
    sway = 0.9 * wind(t, X640)
    dx = hair * sway + cape * lever_c * 1.7 * wind(t, X640)
    dy = (
        hair * 0.35 * wind(t + 0.25, X640)
        + cape * lever_c * 1.1 * wind(t + 0.2, X640 * 1.3)
        - chest * clip01((206.0 - Y640) / 50.0) * 0.6 * breath(t)
    )
    return dx, dy, (200, 66, 510, 252)


def princess_field(t):
    lever_h = clip01((X640 - 528.0) / 80.0) ** 1.1
    hair = protect(
        poly_mask(
            [(530, 103), (560, 106), (586, 122), (610, 138), (624, 165), (622, 196),
             (600, 204), (574, 200), (555, 182), (544, 160), (535, 140)],
            3,
        ),
        PRINCESS_EYES,
    )
    ribbon = poly_mask([(546, 112), (584, 115), (586, 133), (560, 136), (546, 127)], 2)
    left_hair = poly_mask([(478, 124), (499, 124), (499, 192), (476, 192)], 3)
    hem = poly_mask([(468, 280), (640, 280), (640, 320), (468, 320)], 3) * clip01(
        (Y640 - 280.0) / 30.0
    )
    lupine = poly_mask([(592, 280), (640, 280), (640, 360), (592, 360)], 2) * soften(PURPLE, 1.0)
    chest = protect(
        poly_mask([(488, 136), (566, 136), (566, 214), (488, 214)], 10)
        * clip01((Y640 - 134.0) / 8.0),
        PRINCESS_EYES,
    )
    tb = t + 0.37
    dx = (
        hair * lever_h * 1.5 * wind(t + 0.1, X640)
        + ribbon * 1.1 * wind(t + 0.3, X640 * 1.4)
        + left_hair * 0.6 * wind(t + 0.05, X640)
        + hem * 0.7 * wind(t, X640 * 2.0)
        + lupine * clip01((360.0 - Y640) / 70.0) * 1.0 * wind(t + 0.15, X640)
    )
    dy = (
        hair * lever_h * 0.9 * wind(t + 0.35, X640 * 1.5)
        + ribbon * 0.7 * wind(t + 0.55, X640)
        - chest * clip01((214.0 - Y640) / 50.0) * 0.5 * breath(tb)
    )
    return dx, dy, (466, 98, 640, 360)


def canopy_left_field(t):
    m = poly_mask([(0, 0), (316, 0), (316, 64), (0, 64)], 2) * soften(LEAF, 0.6)
    return m * 0.8 * wind(t, X640), m * 0.4 * wind(t + 0.25, X640), (0, 0, 320, 66)


def canopy_right_field(t):
    leaves = poly_mask([(412, 0), (640, 0), (640, 62), (412, 62)], 2) * soften(LEAF, 0.6)
    leaves_side = poly_mask([(596, 96), (640, 96), (640, 150), (596, 150)], 2) * soften(LEAF, 0.6)
    lantern = poly_mask([(546, 26), (588, 26), (588, 100), (546, 100)], 1)
    swing = math.sin(TAU * t) * clip01((Y640 - 30.0) / 62.0)
    dx = (leaves + leaves_side) * 0.8 * wind(t + 0.4, X640) + lantern * 0.9 * swing
    dy = (leaves + leaves_side) * 0.4 * wind(t + 0.65, X640)
    return dx, dy, (408, 0, 640, 152)


def grass_field(t):
    region = poly_mask([(0, 222), (250, 244), (372, 290), (372, 360), (0, 360)], 3)
    m = region * soften(GRASS, 0.5)
    lever = clip01((352.0 - Y640) / 60.0) * 0.7 + 0.3
    return m * lever * 1.1 * wind(t, X640 * 1.2), m * 0.0, (0, 218, 376, 360)


def render_group(name, field, frames, phase_scale=1.0, post=None):
    """Return (bbox, list of RGBA frames cropped to the changed bbox)."""
    full = []
    for i in range(frames):
        t = i / frames
        dx, dy, box = field(t)
        x0, y0, x1, y1 = box
        sx0, sy0 = int(x0 * FX) - 4, int(y0 * FY) - 4
        sx0, sy0 = max(sx0, 0), max(sy0, 0)
        sx1, sy1 = min(int(x1 * FX) + 5, SW), min(int(y1 * FY) + 5, SH)
        crop = SRC[sy0:sy1, sx0:sx1]
        warped = warp(crop, dx[sy0:sy1, sx0:sx1] * FX, dy[sy0:sy1, sx0:sx1] * FY)
        if post is not None:
            warped = post(t, warped, (sx0, sy0, sx1, sy1))
        img = SRC.copy()
        img[sy0:sy1, sx0:sx1] = warped
        full.append(down(img))
    changed = np.zeros((H, W), dtype=bool)
    for f in full:
        changed |= np.abs(f.astype(np.int16) - BASE.astype(np.int16)).max(axis=2) > 3
    ys, xs = np.nonzero(changed)
    bx0, by0, bx1, by1 = xs.min(), ys.min(), xs.max() + 1, ys.max() + 1
    # Every frame is opaque on the *union* of changed pixels: a frame fully
    # replaces that area, so warped and unwarped pixels never mix (which
    # would read as speckle noise around moving edges).
    union = Image.fromarray((changed * 255).astype(np.uint8)).filter(ImageFilter.MaxFilter(3))
    union = np.asarray(union) > 0
    out = []
    for f in full:
        rgba = np.zeros((H, W, 4), dtype=np.uint8)
        rgba[..., :3] = f
        rgba[..., 3] = np.where(union, 255, 0)
        out.append(rgba[by0:by1, bx0:bx1])
    print(f"  {name}: bbox {bx0},{by0} {bx1 - bx0}x{by1 - by0}, {frames} frames, "
          f"{int(changed.sum())} animated px")
    return (int(bx0), int(by0), int(bx1 - bx0), int(by1 - by0)), out


def write_atlas(name, frames, cols):
    h, w = frames[0].shape[:2]
    rows = math.ceil(len(frames) / cols)
    atlas = np.zeros((rows * h, cols * w, 4), dtype=np.uint8)
    for i, f in enumerate(frames):
        r, c = divmod(i, cols)
        atlas[r * h:(r + 1) * h, c * w:(c + 1) * w] = f
    Image.fromarray(atlas, "RGBA").save(os.path.join(OUT, name), optimize=True)
    return cols, rows


# --- Fire ------------------------------------------------------------------

FIRE_FRAMES = 16
FIRE_FPS = 12.0


def fire_field(t):
    lever = clip01((306.0 - Y640) / 52.0) ** 0.9
    m = poly_mask([(388, 238), (466, 238), (466, 312), (388, 312)], 2) * soften(FLAME, 0.8)
    m = m * lever
    dx = m * (1.3 * np.sin(TAU * (t * 2) + Y640 / 6.0) + 0.6 * np.sin(TAU * (t * 3) + X640 / 5.0))
    dy = m * (1.6 * np.sin(TAU * (t * 2) + X640 / 7.0 + 0.7) + 0.8 * np.sin(TAU * t * 5 + Y640 / 4))
    return dx, dy, (386, 232, 470, 314)


def fire_post(t, warped, crop):
    sx0, sy0, sx1, sy1 = crop
    flame = soften(FLAME, 0.6)[sy0:sy1, sx0:sx1][..., None]
    k = 1.0 + 0.09 * math.sin(TAU * 3 * t) + 0.05 * math.sin(TAU * 5 * t + 1.0)
    return warped * (1.0 + (k - 1.0) * flame)


# --- Blinks -----------------------------------------------------------------


def blink_overlays():
    """Hand-placed eyelid pixels at 640x360 (half-closed, closed)."""
    b = BASE.astype(np.int16)

    def px(x, y):
        return b[y, x].copy()

    def shade(c, k):
        return np.clip(c * k, 0, 255)

    res = {}
    # Prince: near eye (lash row 103, eye 104-106, x 443-448), far eye at
    # the face edge (452-453, 107-109).
    skin = (px(444, 108) + px(447, 108) + px(445, 107)) / 3
    far_skin = px(451, 109)
    lash = px(446, 103)
    half = BASE.copy()
    closed = BASE.copy()
    for x in range(443, 449):
        half[103, x] = shade(skin, 0.86)
        half[104, x] = lash
        for y in (103, 104, 105, 106):
            closed[y, x] = shade(skin, 0.9 if y < 105 else 1.0)
        closed[105, x] = lash
    closed[104, 443] = lash
    closed[106, 446] = skin
    closed[106, 447] = skin
    half[106, 446] = skin
    for y in (107, 108, 109):
        closed[y, 452] = shade(far_skin, 0.92)
    closed[108, 452] = lash
    closed[108, 453] = lash
    half[107, 452] = shade(far_skin, 0.9)
    half[108, 452] = lash
    res["prince"] = (half, closed)

    # Princess: near eye lash row 115 (x 511-517), eye 116-118, lower 119;
    # far eye (501-504, 120-124).
    skin = (px(512, 121) + px(515, 121) + px(509, 117)) / 3
    lash = px(514, 115)
    half = BASE.copy()
    closed = BASE.copy()
    for x in range(510, 518):
        half[115, x] = shade(skin, 0.88)
        half[116, x] = lash
        for y in (115, 116, 117, 118, 119):
            closed[y, x] = shade(skin, 0.9 if y < 117 else 1.0)
        closed[118, x] = lash
    closed[117, 517] = lash
    closed[116, 518] = lash
    far_skin = (px(505, 122) + px(505, 125)) / 2
    for x in range(501, 505):
        half[120, x] = shade(far_skin, 0.9)
        half[121, x] = lash
        for y in range(120, 125):
            closed[y, x] = shade(far_skin, 0.92)
        closed[123, x] = lash
    res["princess"] = (half, closed)

    layout = {}
    for who, (half, closed) in res.items():
        changed = (np.abs(half.astype(np.int16) - b).max(axis=2) > 0) | (
            np.abs(closed.astype(np.int16) - b).max(axis=2) > 0
        )
        ys, xs = np.nonzero(changed)
        x0, y0, x1, y1 = xs.min(), ys.min(), xs.max() + 1, ys.max() + 1
        frames = []
        for f in (half, closed):
            rgba = np.zeros((H, W, 4), dtype=np.uint8)
            rgba[..., :3] = f
            rgba[..., 3] = np.where(np.abs(f.astype(np.int16) - b).max(axis=2) > 0, 255, 0)
            frames.append(rgba[y0:y1, x0:x1])
        write_atlas(f"blink_{who}.png", frames, 2)
        layout[who] = (int(x0), int(y0), int(x1 - x0), int(y1 - y0))
        print(f"  blink {who}: {layout[who]}")
    return layout


# --- Static overlays --------------------------------------------------------


def fire_glow():
    w, h = 200, 150
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    d = np.sqrt(((xx - w / 2) / (w / 2)) ** 2 + ((yy - h * 0.62) / (h * 0.55)) ** 2)
    a = np.clip(1.0 - d, 0, 1) ** 2.2
    # Quantize alpha into bands: pixel-art style light falloff.
    a = np.round(a * 6) / 6
    rgba = np.zeros((h, w, 4), dtype=np.uint8)
    rgba[..., 0], rgba[..., 1], rgba[..., 2] = 255, 168, 82
    rgba[..., 3] = (a * 255).astype(np.uint8)
    Image.fromarray(rgba, "RGBA").save(os.path.join(OUT, "fire_glow.png"), optimize=True)
    return w, h


def menu_shade():
    """Left-side darkening strip (stretched vertically at runtime)."""
    w = 300
    x = np.arange(w, dtype=np.float32) / (w - 1)
    a = np.clip(1.0 - x, 0, 1) ** 1.4 * 0.62
    rgba = np.zeros((1, w, 4), dtype=np.uint8)
    rgba[0, :, 0], rgba[0, :, 1], rgba[0, :, 2] = 8, 12, 28
    rgba[0, :, 3] = (a * 255).astype(np.uint8)
    Image.fromarray(rgba, "RGBA").save(os.path.join(OUT, "menu_shade.png"))
    return w


def mist():
    w, h = 240, 48
    rng = np.random.default_rng(7)
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    v = np.zeros((h, w), dtype=np.float32)
    for k, amp in ((1, 1.0), (2, 0.6), (3, 0.35), (5, 0.2)):
        ph = rng.uniform(0, TAU, 2)
        v += amp * np.sin(TAU * k * xx / w + ph[0]) * np.sin(TAU * yy / (h * 2) + ph[1] * 0.2)
    v = (v - v.min()) / (v.max() - v.min())
    env = np.clip(np.sin(math.pi * yy / (h - 1)), 0, 1) ** 1.5
    env = env * np.clip(np.sin(math.pi * xx / (w - 1)), 0, 1) ** 1.2
    a = np.round(v * env * 4) / 4 * 0.5
    rgba = np.zeros((h, w, 4), dtype=np.uint8)
    rgba[..., 0], rgba[..., 1], rgba[..., 2] = 150, 160, 200
    rgba[..., 3] = (a * 255).astype(np.uint8)
    Image.fromarray(rgba, "RGBA").save(os.path.join(OUT, "mist.png"), optimize=True)
    return w, h


# --- Logo -------------------------------------------------------------------

SERIF_BOLD = "/usr/share/fonts/dejavu-serif-fonts/DejaVuSerif-Bold.ttf"
SANS_BOLD = "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf"

GOLD_TOP = np.array([255, 233, 170], dtype=np.float32)
GOLD_MID = np.array([226, 178, 92], dtype=np.float32)
GOLD_BOT = np.array([150, 96, 44], dtype=np.float32)
OUTLINE = np.array([22, 16, 30], dtype=np.float32)


def glyph_mask(text, font, spacing):
    """Aliased glyph run with our own advances (ink width + spacing)."""
    asc, desc = font.getmetrics()
    cell_h = asc + desc
    parts = []
    for ch in text:
        if ch == " ":
            parts.append(np.zeros((cell_h, max(2, font.size // 3)), dtype=bool))
            continue
        im = Image.new("1", (font.size * 2, cell_h), 0)
        d = ImageDraw.Draw(im)
        d.fontmode = "1"
        d.text((2, 0), ch, font=font, fill=1)
        a = np.asarray(im, dtype=bool)
        cols = np.nonzero(a.any(axis=0))[0]
        parts.append(a[:, cols.min():cols.max() + 1])
        parts.append(np.zeros((cell_h, spacing), dtype=bool))
    m = np.concatenate(parts[:-1], axis=1)
    rows = np.nonzero(m.any(axis=1))[0]
    return m[rows.min():rows.max() + 1]


def dilate(m):
    p = np.pad(m, 1)
    out = np.zeros_like(p)
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            out |= np.roll(np.roll(p, dy, 0), dx, 1)
    return out


def logo():
    font = ImageFont.truetype(SERIF_BOLD, 34)
    letters = glyph_mask("CROWNWIND", font, 2)
    lh, lw = letters.shape
    pad = 3
    crown_h = 12
    h = lh + pad * 2 + crown_h + 10
    w = lw + pad * 2
    rgba = np.zeros((h, w, 4), dtype=np.float32)
    top = crown_h + 2 + pad
    mask = np.zeros((h, w), dtype=bool)
    mask[top:top + lh, pad:pad + lw] = letters
    # Gold gradient by row, in three bands with a bright top rim.
    grad = np.zeros((h, 3), dtype=np.float32)
    for y in range(h):
        f = (y - top) / max(lh - 1, 1)
        if f < 0.45:
            c = GOLD_TOP + (GOLD_MID - GOLD_TOP) * (f / 0.45)
        else:
            c = GOLD_MID + (GOLD_BOT - GOLD_MID) * ((f - 0.45) / 0.55)
        grad[y] = np.round(c / 12) * 12
    # Crown ornament centred above the word.
    crown = [
        "......#.....#.....#......",
        ".....###...###...###.....",
        "......#.....#.....#......",
        ".#....##...###...##....#.",
        ".##...###.#####.###...##.",
        ".###..#############..###.",
        "..######################.",
        "..######################.",
        "..#..#..#..#..#..#..#..#.",
        "..######################.",
    ]
    cw = len(crown[0])
    cx = (w - cw) // 2
    for y, row in enumerate(crown):
        for x, ch in enumerate(row):
            if ch == "#":
                mask[1 + y, cx + x] = True
    # Underline flourish: thin rule with a centre diamond.
    uy = top + lh + 4
    for x in range(pad + 6, w - pad - 6):
        if abs(x - w // 2) > 5:
            mask[uy, x] = True
    for d in range(4):
        for x in range(w // 2 - d, w // 2 + d + 1):
            mask[uy - 3 + d, x] = True
            mask[uy + 3 - d, x] = True
    outline = dilate(mask)[1:-1, 1:-1] & ~mask
    # Drop shadow one pixel down-right of the outline.
    shadow = np.zeros_like(mask)
    shadow[1:, 1:] = (mask | outline)[:-1, :-1]
    shadow &= ~(mask | outline)
    for y in range(h):
        for x in range(w):
            if mask[y, x]:
                c = grad[y] if top <= y < top + lh else GOLD_MID
                # Crown jewels / rule get the mid gold, crown top lighter.
                if y < top and y < 4:
                    c = GOLD_TOP
                rgba[y, x, :3] = c
                rgba[y, x, 3] = 255
            elif outline[y, x]:
                rgba[y, x, :3] = OUTLINE
                rgba[y, x, 3] = 255
            elif shadow[y, x]:
                rgba[y, x, :3] = OUTLINE
                rgba[y, x, 3] = 120
    # Jewels on the crown band.
    for jx in (cx + 6, cx + 12, cx + 18):
        rgba[1 + 7, jx, :3] = (64, 196, 190)
    Image.fromarray(rgba.astype(np.uint8), "RGBA").save(os.path.join(OUT, "logo.png"))
    print(f"  logo: {w}x{h}")
    return w, h


# --- Bitmap fonts for the UI -------------------------------------------------

CHARSET = (
    "".join(chr(c) for c in range(33, 127))
    + "ÇĞİÖŞÜçğıöşü·—↑↓←→"
)


def font_table(name, path, size, spacing):
    font = ImageFont.truetype(path, size)
    asc, desc = font.getmetrics()
    cell_h = asc + desc
    glyphs = []
    for ch in CHARSET:
        im = Image.new("1", (size * 2, cell_h + 2), 0)
        d = ImageDraw.Draw(im)
        d.fontmode = "1"
        d.text((2, 0), ch, font=font, fill=1)
        a = np.asarray(im, dtype=bool)
        cols = np.nonzero(a.any(axis=0))[0]
        if len(cols) == 0:
            continue
        g = a[:, cols.min():cols.max() + 1]
        glyphs.append((ch, g))
    # Trim the common empty rows.
    used = np.zeros(cell_h + 2, dtype=bool)
    for _, g in glyphs:
        used |= g.any(axis=1)
    rows = np.nonzero(used)[0]
    r0, r1 = rows.min(), rows.max() + 1
    # Baseline row (relative to r0): bottom of capital "H".
    hrow = [g for c, g in glyphs if c == "H"][0]
    base = np.nonzero(hrow.any(axis=1))[0].max() + 1 - r0
    lines = [f"pub const {name}: PixelFont = PixelFont {{"]
    lines.append(f"    height: {r1 - r0},")
    lines.append(f"    baseline: {base},")
    lines.append(f"    space: {max(2, size // 3)},")
    lines.append(f"    spacing: {spacing},")
    lines.append("    glyphs: &[")
    for ch, g in glyphs:
        g = g[r0:r1]
        rows_bits = []
        for row in g:
            v = 0
            for i, bit in enumerate(row):
                if bit:
                    v |= 1 << i
            rows_bits.append(v)
        lit = repr(ch) if ch not in ("\\", "'") else ("'\\\\'" if ch == "\\" else "'\\''")
        lines.append(f"        Glyph {{ ch: {lit}, width: {g.shape[1]}, rows: &{rows_bits} }},")
    lines.append("    ],")
    lines.append("};")
    return "\n".join(lines)


def write_font_data():
    body = [
        "//! Generated by `tools/menu_art/build_menu_art.py`; do not edit.",
        "//! Aliased bitmap glyphs rasterized from DejaVu Serif Bold / Sans Bold",
        "//! (Bitstream Vera derived license, see `tools/menu_art/README.md`).",
        "",
        "use super::pixel_font::{Glyph, PixelFont};",
        "",
        "/// Button labels.",
        font_table("SERIF", SERIF_BOLD, 12, 1),
        "",
        "/// Small captions (tags, hints).",
        font_table("SANS_SMALL", SANS_BOLD, 9, 1),
        "",
    ]
    with open(os.path.join(RS_DIR, "font_data.rs"), "w") as f:
        f.write("\n".join(body))


# --------------------------------------------------------------------------


def main():
    os.makedirs(OUT, exist_ok=True)
    os.makedirs(RS_DIR, exist_ok=True)
    Image.fromarray(BASE, "RGB").save(os.path.join(OUT, "camp_base.png"), optimize=True)
    print("camp_base.png")

    layout = []
    groups = [
        ("canopy_left", canopy_left_field, LOOP_FRAMES, LOOP_FPS, None, 8),
        ("canopy_right", canopy_right_field, LOOP_FRAMES, LOOP_FPS, None, 8),
        ("grass", grass_field, LOOP_FRAMES, LOOP_FPS, None, 5),
        ("prince", prince_field, LOOP_FRAMES, LOOP_FPS, None, 8),
        ("princess", princess_field, LOOP_FRAMES, LOOP_FPS, None, 8),
        ("fire", fire_field, FIRE_FRAMES, FIRE_FPS, fire_post, 8),
    ]
    only = set(sys.argv[1:])
    for name, field, frames, fps, post, cols in groups:
        if only and name not in only:
            continue
        bbox, imgs = render_group(name, field, frames, post=post)
        cols, rows = write_atlas(f"camp_{name}.png", imgs, cols)
        layout.append((name, bbox, frames, fps, cols, rows))
    if only:
        return
    blinks = blink_overlays()
    gw, gh = fire_glow()
    sw = menu_shade()
    mw, mh = mist()
    lw, lh = logo()
    write_font_data()

    rs = [
        "//! Generated by `tools/menu_art/build_menu_art.py`; do not edit.",
        "//! Camp-scene placement in 640x360 image pixels (origin top-left).",
        "",
        "use super::scene::{Patch, Rect};",
        "",
        "pub const PATCHES: &[Patch] = &[",
    ]
    for name, (x, y, w, h), frames, fps, cols, rows in layout:
        rs.append(
            f"    Patch {{ path: \"menu/camp_{name}.png\", rect: Rect {{ x: {x}, y: {y}, w: {w}, h: {h} }}, "
            f"frames: {frames}, fps: {fps:.1f}, columns: {cols}, rows: {rows} }},"
        )
    rs.append("];")
    for who, (x, y, w, h) in blinks.items():
        rs.append(
            f"pub const BLINK_{who.upper()}: Rect = Rect {{ x: {x}, y: {y}, w: {w}, h: {h} }};"
        )
    rs.append(f"pub const FIRE_GLOW_SIZE: (u32, u32) = ({gw}, {gh});")
    rs.append(f"pub const SHADE_WIDTH: u32 = {sw};")
    rs.append(f"pub const MIST_SIZE: (u32, u32) = ({mw}, {mh});")
    rs.append(f"pub const LOGO_SIZE: (u32, u32) = ({lw}, {lh});")
    rs.append("")
    with open(os.path.join(RS_DIR, "camp_layout.rs"), "w") as f:
        f.write("\n".join(rs))
    print("done")


if __name__ == "__main__":
    main()
