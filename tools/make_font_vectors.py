#!/usr/bin/env python3
"""Make the expected outputs of the font decoder tests (not part of the build).

Needs `pip install fonttools brotli` (test tooling only; the engine has no third-party code).
Usage: make_font_vectors.py <fonts dir> <dictionary.bin> <dictionary output>

* <font>.expected.ttf for every .woff / .woff2 in the fonts directory: the tables of the font as fontTools / zlib
  decode them, laid out the way the engine lays them out (tables sorted by tag, each padded to 4 bytes, the
  checksums of the directory and head.checkSumAdjustment recomputed).
* the Brotli static dictionary, deflate-compressed (raw RFC 1951) for the exe.
"""
import struct, sys, zlib, io, os, glob

def checksum(data):
    data = data + b"\0" * (-len(data) % 4)
    return sum(struct.unpack(">%dI" % (len(data) // 4), data)) & 0xFFFFFFFF

def assemble(flavor, tables):
    tables = sorted(tables.items())
    n = len(tables)
    entry = 0
    while (2 << entry) <= n:
        entry += 1
    search = (1 << entry) * 16
    tables = [(t, bytearray(d)) for t, d in tables]
    off = 12 + 16 * n
    layout = []
    for t, d in tables:
        layout.append((t, off, len(d)))
        off += (len(d) + 3) & ~3
    # head.checkSumAdjustment is zero while the checksums are computed.
    for t, d in tables:
        if t == "head" and len(d) >= 12:
            d[8:12] = b"\0\0\0\0"
    out = bytearray(struct.pack(">IHHHH", flavor, n, search, entry, n * 16 - search))
    for (t, d), (_, o, l) in zip(tables, layout):
        out += struct.pack(">4sIII", t.encode("latin-1"), checksum(bytes(d)), o, l)
    for t, d in tables:
        out += d + b"\0" * (-len(d) % 4)
    total = checksum(bytes(out))
    adj = (0xB1B0AFBA - total) & 0xFFFFFFFF
    for (t, o, l) in layout:
        if t == "head" and l >= 12:
            out[o + 8:o + 12] = struct.pack(">I", adj)
    return bytes(out)

def woff1(path):
    b = open(path, "rb").read()
    sig, flavor, length, n = struct.unpack(">4sIIH", b[:14])
    assert sig == b"wOFF"
    tables = {}
    for i in range(n):
        tag, off, comp, orig, _ = struct.unpack(">4sIIII", b[44 + 20 * i:64 + 20 * i])
        data = b[off:off + comp]
        if comp < orig:
            data = zlib.decompress(data)
        assert len(data) == orig
        tables[tag.decode("latin-1")] = data
    return flavor, tables

def woff2(path):
    from fontTools.ttLib import TTFont
    f = TTFont(path)
    r = f.reader
    return r.flavor if False else struct.unpack(">I", open(path, "rb").read()[4:8])[0], {t: r[t] for t in r.keys()}

fonts, dic, dic_out = sys.argv[1:4]
for p in sorted(glob.glob(os.path.join(fonts, "*.woff")) + glob.glob(os.path.join(fonts, "*.woff2"))):
    flavor, tables = (woff2 if p.endswith("woff2") else woff1)(p)
    open(p.rsplit(".", 1)[0] + "." + p.rsplit(".", 1)[1] + ".expected.ttf", "wb").write(assemble(flavor, tables))
    print(os.path.basename(p), len(tables), "tables")

d = open(dic, "rb").read()
best = None
for level in (9,):
    c = zlib.compressobj(level, zlib.DEFLATED, -15, 9)
    z = c.compress(d) + c.flush()
    if best is None or len(z) < len(best):
        best = z
open(dic_out, "wb").write(best)
print("dictionary", len(d), "->", len(best))
