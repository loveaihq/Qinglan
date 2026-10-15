# Generates crates/qinglan-engine/src/html/entities.bin from docs/spec/entities.json (HTML 13.5 named character references).
# Layout: entries sorted by name (bytewise), each: [len u8][name bytes, without the leading '&'][u24 LE: code point 1 | 1<<23 if a second one follows][u24 LE code point 2, only if flagged].
# Run: python -I tools/gen_entities.py
import json, io
d = json.load(io.open('docs/spec/entities.json', encoding='utf-8'))
out = bytearray()
for k in sorted(d, key=lambda s: s[1:].encode()):
    name = k[1:].encode()
    cps = d[k]['codepoints']
    assert len(cps) in (1, 2) and len(name) < 256 and max(cps) < (1 << 21)
    out.append(len(name)); out += name
    first = cps[0] | ((1 << 23) if len(cps) == 2 else 0)
    out += first.to_bytes(3, 'little')
    if len(cps) == 2:
        out += cps[1].to_bytes(3, 'little')
io.open('crates/qinglan-engine/src/html/entities.bin', 'wb').write(out)
print(len(d), 'entities,', len(out), 'bytes')
