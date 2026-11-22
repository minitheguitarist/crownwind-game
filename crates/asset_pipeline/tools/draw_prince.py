"""Original Crownwind sprite masters. Run with Python 3 from any directory."""
import argparse
import json
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
W, H = 32, 48
# Shared ink, cape, chestnut/leather, skin, steel/linen, and clasp.
COLORS = dict(zip("KVMRBTCSFDAQIE", (
    "292334", "422640", "68314e", "974862", "573435", "88503d", "bf7950",
    "ce896c", "f2c8a0", "48536b", "68869b", "a4c9d2", "f6e8cf", "d8ac66",
)))
RGBA = {key: bytes.fromhex(rgb + "ff") for key, rgb in COLORS.items()}
RGBA["."] = bytes(4)

# Each row is one scanline. Shared masters lock face, collar and belt placement.
HEAD = (
    "....KKKKKK....",
    "..KKBBBTTTKK..",
    ".KBBTTCCCTTTK.",
    ".KBTTTTTTBBTTK",
    "KBBTTCBTBSSBK.",
    "KBTBTBBTSFFFK.",
    "KBBBTBSSFBBFK.",
    ".KBTBSSFIFKFK.",
    ".KBBBSFSFFSFFK",
    "..KBTSFSFFFFK.",
    "..KBBTSSFFBSK.",
    "...KBBKSSSFK..",
    "...KBK.KSSK...",
    "....K...KK....",
)
TUNIC = (
    "...KKKKKK...",
    "..KVMMIIIK..",
    ".KVMMREIQAK.",
    "KVMRQIIQQAK.",
    "KVMQQIQQQAK.",
    ".KMAQIQQAAK.",
    "..KAQIQQADK.",
    "..KAAIQAADK.",
    "..KTBTEBBBK.",
    "..KBCCBQQAK.",
    ".KAQTBQQQAK.",
    ".KAQQQIQQQAK",
    "KDAQQIIQIQQK",
    ".KKDDAAAAKK.",
    "...KKKKKK...",
)


def stamp(grid, x, y, rows):
    for j, row in enumerate(rows, y):
        for i, color in enumerate(row, x):
            if color != ".":
                assert 0 <= i < W and 0 <= j < H
                grid[j][i] = color


def polygon(grid, points, color):
    """Even/odd scan conversion at pixel centers with a half-open right edge."""
    for y in range(min(p[1] for p in points), max(p[1] for p in points)):
        scan = y + 0.5
        cuts = []
        for (x1, y1), (x2, y2) in zip(points, points[1:] + points[:1]):
            if min(y1, y2) <= scan < max(y1, y2):
                cuts.append(x1 + (scan - y1) * (x2 - x1) / (y2 - y1))
        cuts.sort()
        for left, right in zip(cuts[::2], cuts[1::2]):
            for x in range(W):
                if left <= x + 0.5 < right:
                    grid[y][x] = color


def stroke(grid, points, width, color):
    for (x, y), (end_x, end_y) in zip(points, points[1:]):
        steps = max(1, abs(end_x - x), abs(end_y - y))
        for t in range(steps + 1):
            px = x + round((end_x - x) * t / steps)
            py = y + round((end_y - y) * t / steps)
            for j in range(py - width // 2, py + (width + 1) // 2):
                for i in range(px - width // 2, px + (width + 1) // 2):
                    assert 0 <= i < W and 0 <= j < H
                    grid[j][i] = color


def leg(grid, hip, knee, heel, near):
    x, y = heel
    stroke(grid, [hip, knee, heel], 5, "K")
    stroke(grid, [hip, knee], 3, "A" if near else "D")
    stroke(grid, [knee, heel], 3, "T" if near else "B")
    stamp(grid, x - 2, y - 1, ("KCTK.." if near else "KTBK..",
                                "KTTBBK" if near else "KBBBBK", "KKKKKK"))


def cape(grid, x, y, tip, hem):
    # Outer silhouette, plum lining and a light fold all follow the same hem.
    polygon(grid, [(x+4,y),(x+6,y+3),(x+2,y+13),(tip+5,hem-1),
                   (tip+1,hem+2),(tip+1,hem-1),(tip-3,hem),(tip-1,hem-5),
                   (tip+1,hem-8),(x-3,y+3)], "K")
    polygon(grid, [(x+3,y+1),(x+4,y+5),(x,y+12),(tip+3,hem-2),
                   (tip+2,hem),(tip+1,hem-3),(tip-1,hem-2),(tip,hem-5),
                   (tip+2,hem-7),(x-2,y+4)], "M")
    polygon(grid, [(x+1,y+3),(x+2,y+4),(tip+2,hem-5),(tip,hem-3)], "R")
    polygon(grid, [(x+4,y+4),(x+3,y+10),(tip+4,hem-2)], "V")


# torso dx/dy, cape tip x/y, hair sweep, far knee/heel, near knee/heel, elbow/hand.
# Grounded boot soles occupy row 43, immediately above the pivot on y=44.
IDLE = [
    (0,0, 7,34,0), (0,0, 6,33,1), (0,-1, 5,31,2), (0,0, 6,32,1),
]
IDLE = [p + ((14,35),(12,42),(20,35),(20,42),(15,25),(17,29)) for p in IDLE]
RUN = [
    (1,0, 3,28,2, (10,34),(6,40), (22,34),(26,42), (13,24),(11,19)),
    (1,1, 4,27,2, (9,35),(5,36), (22,36),(20,42), (15,25),(13,21)),
    (1,-1, 5,29,1, (20,32),(13,35), (17,35),(15,42), (19,24),(22,22)),
    (1,0, 4,30,1, (23,34),(26,42), (11,34),(6,40), (22,23),(25,19)),
    (1,1, 3,29,2, (22,36),(20,42), (10,35),(5,36), (21,24),(24,21)),
    (1,-1, 3,27,2, (17,35),(15,42), (21,32),(14,35), (17,25),(20,23)),
]
JUMP = [
    (1,2, 6,33,0, (12,36),(10,42), (23,35),(20,42), (13,26),(10,24)),
    (1,-2, 7,36,1, (12,35),(8,38), (23,29),(21,35), (21,19),(24,14)),
]
FALL = [
    (0,-1, 3,25,2, (11,33),(10,38), (21,33),(21,39), (12,21),(9,18)),
    (0,0, 4,22,2, (12,35),(11,39), (20,35),(21,41), (11,21),(8,17)),
]


def frame(pose):
    dx, dy, tip, hem, sweep, far_k, far_h, near_k, near_h, elbow, hand = pose
    grid = [["."] * W for _ in range(H)]
    x, y = 13 + dx, 17 + dy
    leg(grid, (x+3,y+12), far_k, far_h, False)
    leg(grid, (x+5,y+12), near_k, near_h, True)
    cape(grid, x, y, tip, hem)
    # Opposing arm swing reads beyond the chest in the rearward near-arm poses.
    far_elbow = (2*x+8-elbow[0], elbow[1])
    far_hand = (min(W-3, 2*x+8-hand[0]), hand[1]+1)
    stroke(grid, [(x+5,y+3), far_elbow, far_hand], 3, "K")
    stroke(grid, [(x+5,y+3), far_elbow], 2, "A")
    stroke(grid, [far_elbow, far_hand], 1, "T")
    stamp(grid, far_hand[0]-1, far_hand[1]-1, ("SK","TS"))
    stamp(grid, x-2, y, TUNIC)
    shoulder = (x+1,y+3)
    stroke(grid, [shoulder, elbow, hand], 4, "K")
    stroke(grid, [shoulder, elbow], 3, "Q")
    stroke(grid, [elbow, hand], 2, "T")
    stamp(grid, hand[0]-1, hand[1]-1, ("SK","FS"))
    # Trailing lock is anchored underneath the same head in every pose.
    polygon(grid, [(x+3,y-8),(x+3,y-2),(x-2-sweep,y+1-sweep),
                   (x-1-sweep,y-2-sweep),(x-3-sweep,y-2-sweep)], "K")
    polygon(grid, [(x+2,y-7),(x+2,y-3),(x-1-sweep,y-sweep),
                   (x-sweep,y-3-sweep)], "T")
    stamp(grid, x-1, y-12, HEAD)
    return grid


def png(rows):
    def chunk(tag, data):
        return (struct.pack(">I", len(data)) + tag + data
                + struct.pack(">I", zlib.crc32(tag + data)))

    header = struct.pack(">2I5B", len(rows[0])//4, len(rows), 8, 6, 0, 0, 0)
    pixels = b"".join(b"\0" + row for row in rows)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header)
            + chunk(b"IDAT", zlib.compress(pixels, 9)) + chunk(b"IEND", b""))


# Small hand-lettered row labels in the review sheet.
FONT = {
    "A":"010/101/111/101/101", "D":"110/101/101/101/110", "E":"111/100/110/100/111",
    "F":"111/100/110/100/100", "I":"111/010/010/010/111", "J":"011/001/001/101/010",
    "L":"100/100/100/100/111", "M":"10001/11011/10101/10101/10001",
    "N":"1001/1101/1011/1001/1001", "P":"110/101/110/100/100", "R":"110/101/110/101/101",
    "U":"101/101/101/101/111",
}


def preview(groups):
    paper = bytes.fromhex("e1dcd0ff")
    panel = bytes.fromhex("f0ece3ff")
    rows = [[paper] * 242 for _ in range(4 * 52)]
    for row, (name, frames) in enumerate(groups):
        x, y = 5, row * 52
        for letter in name:
            glyph = FONT[letter].split("/")
            for j, scan in enumerate(glyph):
                for i, bit in enumerate(scan):
                    if bit == "1":
                        rows[y+24+j][x+i] = RGBA["K"]
            x += len(glyph[0]) + 1
        for col, cell in enumerate(frames):
            x = 27 + col * 36
            for j, scan in enumerate(cell):
                rows[y+2+j][x:x+W] = [panel if c == "." else RGBA[c] for c in scan]
    return png([b"".join(pixel * 3 for pixel in row) for row in rows for _ in range(3)])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="compare generated bytes with disk")
    args = parser.parse_args()
    groups = [(name, list(map(frame, poses))) for name, poses in
              [("IDLE", IDLE), ("RUN", RUN), ("JUMP", JUMP), ("FALL", FALL)]]
    manifest = json.loads((ROOT / "content/characters/prince_debug.json").read_text())
    if (manifest["frame_width"], manifest["frame_height"], manifest["pivot"]) != (W, H, {"x": 16, "y": 44}):
        raise SystemExit("prince_debug dimensions or pivot differ from the drawing master")
    offset = 0
    expected = {}
    for name, frames in groups:
        expected[name.lower()] = list(range(offset, offset + len(frames)))
        offset += len(frames)
    if expected != {k: v["frames"] for k, v in manifest["animations"].items()}:
        raise SystemExit("prince_debug state indices differ from the drawn poses")
    cells = [cell for _, frames in groups for cell in frames]
    assert len({tuple(map(tuple, cell)) for cell in cells}) == 14
    atlas = png([b"".join(RGBA[c] for cell in cells for c in cell[j]) for j in range(H)])
    for name, data in [("characters/prince_debug.png", atlas),
                       ("previews/prince_debug.png", preview(groups))]:
        path = ROOT / "assets" / name
        if args.check:
            if not path.is_file() or path.read_bytes() != data:
                raise SystemExit(f"Stale or missing: {path}")
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)


if __name__ == "__main__":
    main()
