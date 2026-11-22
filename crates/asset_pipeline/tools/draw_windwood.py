"""Render the original Windwood tiles, parallax plates and composition."""
import argparse
from math import ceil
from random import Random

from draw_prince import IDLE, RGBA, ROOT, frame, png

CLEAR = bytes(4)
INK = "292334"
SOIL = ("382c37", "573435", "71463f", "885847")
STONE = ("303342", "48536b", "606c78", "798387")
MOSS = ("334638", "4b6040", "718151", "a0a15f")


class Canvas:
    def __init__(self, width, height):
        self.width, self.height = width, height
        self.rows = [[CLEAR] * width for _ in range(height)]

    def box(self, x, y, w, h, color):
        pixel = bytes.fromhex(color + "ff")
        left, right = max(0, x), min(self.width, x + w)
        if left < right:
            for j in range(max(0, y), min(self.height, y + h)):
                self.rows[j][left:right] = [pixel] * (right - left)

    def poly(self, points, color):
        edges = list(zip(points, points[1:] + points[:1]))
        for y in range(max(0, min(p[1] for p in points)),
                       min(self.height, max(p[1] for p in points))):
            scan = y + 0.5
            cuts = sorted(x + (scan - a) * (u - x) / (b - a)
                          for (x, a), (u, b) in edges if min(a, b) <= scan < max(a, b))
            for left, right in zip(cuts[::2], cuts[1::2]):
                start, end = ceil(left - 0.5), ceil(right - 0.5)
                self.box(start, y, end - start, 1, color)

    def paste(self, source, x, y):
        for j, row in enumerate(source.rows, y):
            for i, px in enumerate(row, x):
                if px[3] and 0 <= i < self.width and 0 <= j < self.height:
                    self.rows[j][i] = px

    def encode(self):
        return png([b"".join(row) for row in self.rows])


def earth(seed, grass=False, edge=None):
    tile = Canvas(16, 16)
    rng = Random(seed)
    tile.box(0, 0, 16, 16, SOIL[1])
    for _ in range(15):
        x, y = rng.randrange(16), rng.randrange(16)
        tile.poly([(x,y),(x+4,y),(x+5,y+2),(x+2,y+3),(x,y+2)],
                  rng.choice((SOIL[0], SOIL[2], SOIL[3])))
    if grass:
        tile.poly([(0,0),(16,0),(16,5),(13,5),(12,8),(10,6),
                   (7,6),(6,9),(4,7),(2,6),(0,6)], MOSS[0])
        tile.poly([(0,0),(16,0),(16,3),(12,3),(10,5),(7,4),
                   (5,6),(3,4),(0,4)], MOSS[2])
        for x, y, w in [(0,0,4),(6,1,3),(10,0,3),(14,1,2)]:
            tile.box(x,y,w,1,MOSS[3])
    if edge:
        tile.poly([(0,0),(2,3),(1,6),(4,10),(5,16),(0,16)], INK)
        for y, clear in enumerate([0,0,0,1,1,1,0,1,1,2,2,3,3,3,4,4]):
            tile.rows[y][:clear] = [CLEAR] * clear
        if edge == "right":
            tile.rows = [row[::-1] for row in tile.rows]
    return tile


def masonry(moss=False):
    tile = Canvas(16, 16)
    tile.box(0,0,16,16,STONE[0])
    for x,y,w,h in [(0,0,9,7),(10,0,6,7),(-3,8,8,7),(6,8,10,7)]:
        tile.poly([(x,y+1),(x+2,y),(x+w-1,y),(x+w,y+2),
                   (x+w,y+h-1),(x+1,y+h),(x,y+h-2)], STONE[1])
        tile.box(x+2,y+1,w-3,1,STONE[3])
        tile.box(x+1,y+2,w-3,2,STONE[2])
        tile.box(x+w-3,y+4,2,1,STONE[0])
    if moss:
        tile.poly([(0,0),(16,0),(16,3),(13,4),(9,3),(8,6),(5,4),(0,4)], MOSS[0])
        tile.box(0,0,7,2,MOSS[2])
        tile.box(10,1,5,1,MOSS[2])
    return tile


def ornaments():
    fern, roots, flower, thorns = [Canvas(16,16) for _ in range(4)]
    fern.poly([(7,16),(7,10),(1,7),(0,4),(5,5),(8,8),(9,3),(13,0),
               (15,1),(12,5),(11,9),(15,7),(16,9),(11,13),(9,16)], MOSS[0])
    fern.poly([(1,5),(5,6),(8,10),(9,7),(11,3),(14,1),(12,5),
               (10,10),(9,15),(8,15),(8,11),(4,8)], MOSS[2])
    roots.poly([(0,1),(3,0),(7,4),(9,7),(16,6),(16,9),(10,10),
                (8,15),(6,16),(7,11),(5,8),(0,5)], INK)
    roots.poly([(0,2),(3,1),(7,5),(9,8),(15,7),(15,8),(9,9),
                (7,14),(7,10),(4,6),(0,4)], SOIL[3])
    for x,y in [(4,8),(11,4),(13,10)]:
        flower.box(x,y,1,16-y,MOSS[1])
        flower.box(x-2,y,5,2,"a6676f")
        flower.box(x-1,y-1,3,3,"ce896c")
        flower.box(x,y,1,1,"f6e8cf")
    thorns.box(0,13,16,3,INK)
    for x,y in [(0,5),(5,2),(10,4)]:
        thorns.poly([(x,15),(x+3,y),(x+6,15)], INK)
        thorns.poly([(x+2,12),(x+3,y+2),(x+5,13)], "974862")
        thorns.box(x+3,y+3,1,2,"f2c8a0")
    return fern, roots, flower, thorns


def distance():
    sky = Canvas(480,270)
    for y,h,c in [(0,52,"647b84"),(52,36,"7e9295"),(88,31,"a0a69a"),
                  (119,46,"b6b4a0"),(165,105,"a5aa9d")]:
        sky.box(0,y,480,h,c)
    for x,y,w,h in [(312,35,22,3),(305,38,36,6),(301,44,44,22),
                    (305,66,36,6),(312,72,22,3)]:
        sky.box(x,y,w,h,"e4cfaa")
    rng = Random(27)
    for y in (36,58,83,113):
        for _ in range(5):
            x = rng.randrange(480)
            sky.box(x,y+rng.randrange(8),rng.randrange(12,58),1,"acb5aa")
    sky.poly([(0,137),(35,111),(69,82),(90,92),(101,107),(144,120),(174,93),
              (213,59),(228,71),(251,108),(286,138),(340,111),(371,83),
              (392,107),(418,116),(466,72),(480,85),(480,270),(0,270)],"788d8d")
    for points in [[(46,102),(69,82),(90,92),(101,116),(73,103),(65,111)],
                   [(187,85),(213,59),(228,71),(245,102),(221,85),(213,91)],
                   [(350,102),(371,83),(392,107),(374,99),(365,104)],
                   [(444,92),(466,72),(480,85),(480,96),(466,88)]]:
        sky.poly(points,"a6aaa0")
    sky.poly([(0,153),(46,135),(83,139),(120,121),(161,151),(192,149),
              (234,120),(264,133),(290,156),(333,145),(360,130),(417,158),
              (456,135),(480,142),(480,270),(0,270)],"697f7d")
    for x,y,w in [(0,147,101),(81,151,145),(209,155,163),(354,149,126),(0,172,480)]:
        sky.box(x,y,w,2,"939f94")
    return sky


def trees():
    forest = Canvas(480,270)
    rng = Random(84)
    for baseline, step, scale, colors in [
        (226,15,1,("596f6c","647977")),
        (270,29,2,("394f51","485e5c")),
    ]:
        forest.box(0,baseline-21,480,271-baseline+21,colors[0])
        for x in range(-8,490,step):
            x += rng.randrange(-4,5)
            top = baseline - rng.randrange(39,70)*scale
            forest.box(x-2*scale,top,4*scale,baseline-top,colors[0])
            for tier in range(5):
                y = top + tier*9*scale
                width = (5+tier*2)*scale
                forest.poly([(x,y-8*scale),(x+width,y+9*scale),
                             (x+width//3,y+8*scale),(x,y+12*scale),
                             (x-width,y+9*scale)], colors[0])
                forest.poly([(x,y-3*scale),(x,y+6*scale),(x-width+scale,y+8*scale)],
                            colors[1])
    return forest


def render():
    tiles = [earth(2,True), earth(7), earth(18), masonry(), masonry(True),
             earth(10,True,"left"), earth(11,True,"right"), *ornaments()]
    atlas = Canvas(16*len(tiles),16)
    for i, tile in enumerate(tiles):
        assert (tile.width,tile.height) == (16,16)
        atlas.paste(tile, i*16, 0)
    assert len(tiles) == 11
    for tile in tiles:
        assert any(px != CLEAR for row in tile.rows for px in row)
    for tile in tiles[5:]:
        assert any(px == CLEAR for row in tile.rows for px in row)
    sky, forest = distance(), trees()
    scene = Canvas(480,270)
    scene.paste(sky,0,0)
    scene.paste(forest,0,0)
    for left,right,row,surface in [(0,8,13,0),(10,14,11,4),(16,21,9,0),(23,30,12,4),
                                    (8,23,16,1)]:
        for col in range(left,right):
            for r in range(row,17):
                if r > row and left in (10,16) and r > row+1:
                    continue
                tile = surface if r == row else (1+(col+r)%2 if surface != 4 else 3)
                if r == row and surface == 0 and col in (left,right-1):
                    tile = 5 if col == left else 6
                scene.paste(tiles[tile],16*col,16*r)
    for tile,x,y in [(7,2,12),(9,6,12),(8,4,14),(7,12,10),(9,18,8),
                     (8,18,10),(7,25,11),(10,27,11),(10,28,11),(9,29,11)]:
        scene.paste(tiles[tile],16*x,16*y)
    prince = Canvas(32,48)
    prince.rows = [[RGBA[c] for c in row] for row in frame(IDLE[2])]
    scene.paste(prince, 92-16, 208-44)
    assert all((layer.width,layer.height) == (480,270) for layer in (sky,forest,scene))
    return {"environment/windwood/tiles.png": atlas,
            "environment/windwood/sky.png": sky,
            "environment/windwood/forest.png": forest,
            "previews/windwood.png": scene}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name, canvas in render().items():
        path = ROOT / "assets" / name
        data = canvas.encode()
        if args.check:
            if not path.is_file() or path.read_bytes() != data:
                raise SystemExit(f"Stale or missing: {path}")
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)


if __name__ == "__main__":
    main()
