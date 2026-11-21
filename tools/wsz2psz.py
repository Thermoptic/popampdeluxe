#!/usr/bin/env python3
"""Convert a classic Winamp 2 skin (.wsz, or an unpacked folder) to a Popamp
vector skin (.psz).

Every skin bitmap becomes an SVG sheet of the same size. The picture is cut
into flat-coloured areas: the commonest colour is one rectangle under
everything, every other colour is one <path> of merged rectangles (runs of
equal pixels joined across rows). Edges are drawn crisp, so the skin is
exact at x1, x2, x3 and shows no seams between areas at fractional sizes.

    python tools/wsz2psz.py Winamp_base-2.91.wsz [out.psz]

Standard library only (Python 3.7+).
"""
import collections, os, re, struct, sys, zipfile

SHEETS = ["main", "cbuttons", "titlebar", "shufrep", "text", "volume", "balance", "monoster",
          "playpaus", "numbers", "nums_ex", "posbar", "eqmain", "eq_ex", "pledit"]


def rle8(d, off, w, h):
    rows = [[0] * w for _ in range(h)]
    x = y = 0
    i = off
    while i + 1 < len(d) and y < h:
        n, v = d[i], d[i + 1]
        i += 2
        if n:
            for _ in range(n):
                if x < w:
                    rows[y][x] = v
                x += 1
        elif v == 0:
            x, y = 0, y + 1
        elif v == 1:
            break
        elif v == 2:
            x, y = x + d[i], y + d[i + 1]
            i += 2
        else:
            for k in range(v):
                if x < w:
                    rows[y][x] = d[i + k]
                x += 1
            i += v + (v & 1)
    return rows


def read_bmp(path):
    """Rows (top first) of (r, g, b) tuples."""
    d = open(path, "rb").read()
    off = struct.unpack("<I", d[10:14])[0]
    hs, w, h, _, bpp, comp = struct.unpack("<IiiHHI", d[14:34])
    top_down = h < 0
    h = abs(h)
    pal = []
    if bpp <= 8:
        n = struct.unpack("<I", d[46:50])[0] or (1 << bpp)
        pal = [(d[14 + hs + 4 * k + 2], d[14 + hs + 4 * k + 1], d[14 + hs + 4 * k]) for k in range(n)]
    if bpp == 8 and comp == 1:
        idx = rle8(d, off, w, h)
        rows = [[pal[min(v, len(pal) - 1)] for v in r] for r in idx]
    else:
        stride = ((w * bpp + 31) // 32) * 4
        rows = []
        for y in range(h):
            o = off + y * stride
            if bpp == 8:
                r = [pal[min(d[o + x], len(pal) - 1)] for x in range(w)]
            elif bpp == 4:
                r = [pal[(d[o + x // 2] >> (4 if x % 2 == 0 else 0)) & 15] for x in range(w)]
            elif bpp == 1:
                r = [pal[(d[o + x // 8] >> (7 - x % 8)) & 1] for x in range(w)]
            elif bpp in (24, 32):
                s = bpp // 8
                r = [(d[o + x * s + 2], d[o + x * s + 1], d[o + x * s]) for x in range(w)]
            else:
                raise ValueError("%s: %d bit BMP is not supported" % (path, bpp))
            rows.append(r)
    if not top_down:
        rows.reverse()
    return rows


def hexc(c):
    return "#%02x%02x%02x" % c


def to_svg(rows):
    h, w = len(rows), len(rows[0])
    count = collections.Counter(c for r in rows for c in r)
    bg = count.most_common(1)[0][0]
    # Merge equal runs vertically: open[(x0, x1, colour)] = start row.
    rects = collections.defaultdict(list)
    open_ = {}
    for y in range(h + 1):
        runs = set()
        if y < h:
            x = 0
            r = rows[y]
            while x < w:
                x1 = x
                while x1 + 1 < w and r[x1 + 1] == r[x]:
                    x1 += 1
                if r[x] != bg:
                    runs.add((x, x1 + 1, r[x]))
                x = x1 + 1
        for k in [k for k in open_ if k not in runs]:
            rects[k[2]].append((k[0], open_.pop(k), k[1] - k[0], y - 0))
        for k in runs:
            open_.setdefault(k, y)
    out = ['<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d" shape-rendering="crispEdges">' % (w, h, w, h),
           '<rect width="%d" height="%d" fill="%s"/>' % (w, h, hexc(bg))]
    for c, lst in sorted(rects.items(), key=lambda kv: -sum(r[2] * (r[3] - r[1]) for r in kv[1])):
        d = "".join("M%d %dh%dv%dh-%dz" % (x, y0, ww, y1 - y0, ww) for x, y0, ww, y1 in lst)
        out.append('<path fill="%s" d="%s"/>' % (hexc(c), d))
    out.append("</svg>")
    return "\n".join(out) + "\n"


def parse_ini(text):
    sec, res = "", {}
    for line in text.replace("\r", "").split("\n"):
        line = line.split(";")[0].strip()
        m = re.match(r"\[(.+)\]", line)
        if m:
            sec = m.group(1).lower()
        elif "=" in line:
            k, v = line.split("=", 1)
            res[(sec, k.strip().lower())] = v.strip()
    return res


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    src = sys.argv[1]
    if os.path.isdir(src):
        files = {f.lower(): os.path.join(src, f) for f in os.listdir(src)}
        read = lambda p: open(p, "rb").read()
    else:
        z = zipfile.ZipFile(src)
        files = {os.path.basename(n).lower(): n for n in z.namelist() if not n.endswith("/")}
        read = lambda n: z.read(n)
    name = re.sub(r"\.(wsz|zip)$", "", os.path.basename(os.path.normpath(src)), flags=re.I)
    out = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(os.path.abspath(src)), name + ".psz")

    import tempfile
    tmp = tempfile.mkdtemp()
    parts = {}
    for s in SHEETS:
        f = files.get(s + ".bmp")
        if not f:
            print("no %s.bmp: the default skin's sheet is used" % s)
            continue
        p = os.path.join(tmp, s + ".bmp")
        open(p, "wb").write(read(f))
        svg = to_svg(read_bmp(p))
        parts["sheets/%s.svg" % s] = svg
        print("%-9s %6d bytes" % (s, len(svg)))

    toml = ['[meta]', 'name = "%s"' % name.replace("_", " "), 'author = "Nullsoft (vector conversion by wsz2psz)"',
            'format_version = 1', '', '[sheets]']
    toml += ['%s = "sheets/%s.svg"' % (s, s) for s in SHEETS if "sheets/%s.svg" % s in parts]
    pl = files.get("pledit.txt")
    if pl:
        ini = parse_ini(read(pl).decode("latin-1"))
        toml += ["", "[playlist]"]
        for key, k in [("normal", "normal"), ("current", "current"), ("normal_bg", "normalbg"), ("selected_bg", "selectedbg")]:
            v = ini.get(("text", k))
            if v and re.match(r"#?[0-9a-fA-F]{6}$", v):
                toml.append('%s = "#%s"' % (key, v.lstrip("#").lower()))
    vc = files.get("viscolor.txt")
    if vc:
        cols = []
        for line in read(vc).decode("latin-1").replace("\r", "").split("\n"):
            m = re.match(r"\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)", line)
            if m and len(cols) < 24:
                cols.append('"#%02x%02x%02x"' % tuple(min(255, int(g)) for g in m.groups()))
        if len(cols) == 24:
            toml += ["", "[vis]", "colors = ["] + ["    " + ", ".join(cols[i:i + 8]) + "," for i in range(0, 24, 8)] + ["]"]
    parts["skin.toml"] = "\n".join(toml) + "\n"

    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("skin.toml", parts.pop("skin.toml"))
        for n, t in parts.items():
            z.writestr(n, t)
    print("wrote", out, os.path.getsize(out), "bytes")


if __name__ == "__main__":
    main()
