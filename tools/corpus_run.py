#!/usr/bin/env python3
"""Real-page corpus runner for qinglan (step 1c-3). Test tooling only, not product code.

Stages (run them in this order; each takes optional page names):
  record   qinglan-cli record <url> tests/corpus/local/<name>      (skipped when the recording exists)
  render   qinglan-cli render ... --replay <dir> --width 1280 --height 2400  -> tests/out/<name>-ours.png
  edge     Edge headless, JS off, same size, isolated empty --user-data-dir   -> tests/out/<name>-edge.png
           (`--keep-edge` skips the capture and keeps the existing <name>-edge.png: the live pages change from day to day,
           which makes a new capture differ from the recording; the other stages are not affected)
  compose  ours | edge side by side, scaled down                        -> tests/out/<name>.png
  check    text completeness/order (layout text vs DOM text), tofu, U+FFFD, broken images
  bench    qinglan-cli bench x3 at 1920x1080: first screen, full layout, private bytes after scrolling
  report   tests/out/results.json -> PAGES.md and the corpus table (stdout)

Needs PIL (pip) for compose. The Edge run never touches the user's own profile.
"""
import concurrent.futures as cf
import datetime
import json
import os
import re
import shutil
import statistics
import time
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "target" / "release" / "qinglan-cli.exe"
CORPUS = ROOT / "tests" / "corpus" / "local"
OUT = ROOT / "tests" / "out"
RESULTS = OUT / "results.json"
EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
W, H = 1280, 2400

# (name, category, url)
PAGES = [
    ("zhwiki-rec", "中文维基（长条目，性能页，旧录制）", "https://zh.wikipedia.org/wiki/%E4%B8%AD%E5%8D%8E%E4%BA%BA%E6%B0%91%E5%85%B1%E5%92%8C%E5%9B%BD"),
    ("zhwiki-beijing", "中文维基（长条目）", "https://zh.wikipedia.org/wiki/%E5%8C%97%E4%BA%AC%E5%B8%82"),
    ("zhwiki-rust", "中文维基", "https://zh.wikipedia.org/wiki/Rust"),
    ("zhwiki-apple", "中文维基（短条目）", "https://zh.wikipedia.org/wiki/%E8%8B%B9%E6%9E%9C"),
    ("enwiki-rust", "英文维基", "https://en.wikipedia.org/wiki/Rust_(programming_language)"),
    ("enwiki-hk", "英文维基（长条目）", "https://en.wikipedia.org/wiki/Hong_Kong"),
    ("enwiki-popul", "英文维基（大表格）", "https://en.wikipedia.org/wiki/List_of_countries_and_dependencies_by_population"),
    ("enwiki-main", "英文维基（首页）", "https://en.wikipedia.org/wiki/Main_Page"),
    ("ruanyifeng-weekly", "中文博客", "https://www.ruanyifeng.com/blog/2026/10/weekly-issue-415.html"),
    ("ruanyifeng-home", "中文博客（首页）", "https://www.ruanyifeng.com/blog/"),
    ("codingnow-blog", "中文博客（首页）", "https://blog.codingnow.com/"),
    ("lilianweng", "静态博客（Hugo，英文长文）", "https://lilianweng.github.io/"),
    ("chinanews-art", "中文新闻正文", "https://www.chinanews.com.cn/gn/2026/10-15/10715019.shtml"),
    ("people-art", "中文新闻正文", "http://politics.people.com.cn/n1/2026/0926/c1024-40806006.html"),
    ("xinhua-art", "中文新闻正文", "https://www.news.cn/local/20261015/7b544534c7424aea9f7fd7d1e5b4c6da/c.html"),
    ("bbc-zhongwen", "中文新闻正文", "https://www.bbc.com/zhongwen/articles/cm2d65npyrdjo/simp"),
    ("mdn-div", "技术文档（MDN）", "https://developer.mozilla.org/en-US/docs/Web/HTML/Element/div"),
    ("mdn-flex", "技术文档（MDN）", "https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_flexible_box_layout/Basic_concepts_of_flexbox"),
    ("rust-book-own", "技术文档（Rust）", "https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html"),
    ("rust-std-vec", "技术文档（Rust，API 页）", "https://doc.rust-lang.org/std/vec/struct.Vec.html"),
    ("python-tutorial", "技术文档（Python）", "https://docs.python.org/3/tutorial/introduction.html"),
    ("python-json", "技术文档（Python）", "https://docs.python.org/3/library/json.html"),
    ("python-zhcn", "技术文档（Python 中文）", "https://docs.python.org/zh-cn/3/tutorial/introduction.html"),
    ("gh-ripgrep", "GitHub 仓库首页", "https://github.com/BurntSushi/ripgrep"),
    ("gh-rust", "GitHub 仓库首页", "https://github.com/rust-lang/rust"),
    ("so-undo-commit", "Stack Overflow 问答（被 Cloudflare 拦，403）", "https://stackoverflow.com/questions/927358/how-do-i-undo-the-most-recent-local-commits-in-git"),
    ("rustusers-topic", "问答页（Discourse 论坛）", "https://users.rust-lang.org/t/welcome-to-the-rust-programming-language-users-forum/77411"),
    ("v2ex-topic", "问答页（中文论坛主题）", "https://www.v2ex.com/t/1234"),
    ("hn-front", "Hacker News（表格排版）", "https://news.ycombinator.com/"),
    ("hn-item", "Hacker News（评论页）", "https://news.ycombinator.com/item?id=50107379"),
    ("zol-gbk", "GBK 老网站", "http://www.zol.com.cn/"),
    ("pconline-gbk", "GBK 老网站", "http://www.pconline.com.cn/"),
    ("nthu-big5", "Big5 网站", "http://www.cs.nthu.edu.tw/~king/"),
    ("aozora-sjis", "日文网站（Shift_JIS）", "https://www.aozora.gr.jp/cards/000148/files/773_14560.html"),
    ("jawiki-japan", "日文网站（日文维基长条目）", "https://ja.wikipedia.org/wiki/%E6%97%A5%E6%9C%AC"),
    ("asahi-home", "日文网站（新闻首页）", "https://www.asahi.com/"),
    ("gov-cn-home", "政府网站", "https://www.gov.cn/"),
    ("gov-cn-art", "政府网站（正文）", "https://www.gov.cn/yaowen/liebiao/202610/content_7083319.htm"),
    ("gov-uk", "政府网站", "https://www.gov.uk/"),
    ("usa-gov", "政府网站", "https://www.usa.gov/"),
    ("kantei-jp", "政府网站（日本）", "https://www.kantei.go.jp/"),
    ("matklad", "静态博客（Jekyll）", "https://matklad.github.io/"),
    ("jvns", "静态博客（Hugo）", "https://jvns.ca/"),
    ("gohugo", "静态博客（Hugo 文档）", "https://gohugo.io/"),
    ("jekyllrb", "静态博客（Jekyll 文档）", "https://jekyllrb.com/"),
    ("danluu", "静态博客（纯手写）", "https://danluu.com/"),
    ("yihui", "静态博客（Hugo，中英）", "https://yihui.org/"),
    ("example", "极简页面", "https://example.com/"),
    ("gnu-gpl3", "长文本（许可证）", "https://www.gnu.org/licenses/gpl-3.0.html"),
]
BY_NAME = {p[0]: p for p in PAGES}


def load() -> dict:
    return json.loads(RESULTS.read_text(encoding="utf-8")) if RESULTS.exists() else {}


def save(res: dict) -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    RESULTS.write_text(json.dumps(res, ensure_ascii=False, indent=1), encoding="utf-8")


def run(args, timeout, **kw):
    """Run a command with a hard timeout; on timeout kill the whole tree of *that* process only."""
    p = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, **kw)
    try:
        out, err = p.communicate(timeout=timeout)
        return p.returncode, out.decode("utf-8", "replace"), err.decode("utf-8", "replace"), False
    except subprocess.TimeoutExpired:
        subprocess.run(["taskkill", "/PID", str(p.pid), "/T", "/F"], capture_output=True)
        out, err = p.communicate()
        return -1, out.decode("utf-8", "replace"), err.decode("utf-8", "replace"), True


def first_url(name: str):
    idx = CORPUS / name / "index.txt"
    if not idx.exists():
        return None, None
    status = None
    for line in idx.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("url: "):
            url = line[5:].strip()
        elif line.startswith("status: ") and status is None:
            status = int(line[8:].strip() or 0)
            return url, status
    return None, None


def record_one(name):
    _, _, url = BY_NAME[name]
    d = CORPUS / name
    if (d / "index.txt").exists():
        return name, "exists"
    rc, out, err, to = run([str(CLI), "record", url, str(d), "--width", str(W), "--height", str(H)], 300)
    u, st = first_url(name)
    return name, f"rc={rc} timeout={to} status={st} | {out.strip().splitlines()[-3:] if out else ''} {err.strip()[-200:]}"


def render_one(name):
    u, st = first_url(name)
    if not u:
        return name, None
    png = OUT / f"{name}-ours.png"
    rc, out, err, to = run([str(CLI), "render", u, "--replay", str(CORPUS / name), "-o", str(png), "--width", str(W), "--height", str(H)], 90)
    return name, {"rc": rc, "timeout": to, "stdout": out.strip(), "stderr": err.strip()[-400:]}


def sweep(prof_name: str) -> int:
    """Stop Edge processes started with *our* profile folder (found by command line); nothing else."""
    ps = ("Get-CimInstance Win32_Process -Filter \"Name='msedge.exe'\" | Where-Object { $_.CommandLine -like '*%s*' } "
          "| ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue; $_.ProcessId }" % prof_name)
    rc, out, err, to = run(["powershell", "-NoProfile", "-Command", ps], 30)
    return len(out.split())


def edge_one(name):
    """Edge headless, JS off, same size. `--blink-settings=scriptEnabled=false` (the task's flag) makes this Edge (155) hang
    without writing the screenshot, so JS is switched off in the *new, empty* profile's content settings instead (checked: a
    script that rewrites the page does not run, <noscript> content shows). The user's own profile is never touched."""
    _, _, url = BY_NAME[name]
    final = OUT / f"{name}-edge.png"
    final.unlink(missing_ok=True)
    prof = Path(tempfile.mkdtemp(prefix="qinglan-edge-", dir=os.environ["QL_SCRATCH"]))
    (prof / "Default").mkdir()
    (prof / "Default" / "Preferences").write_text(json.dumps({"profile": {"default_content_setting_values": {"javascript": 2}}}))
    # Edge could not write into this repository's folder; the screenshot goes next to the profile, then is copied.
    png = prof.parent / f"{prof.name}.png"
    # The launcher returns at once, the browser carries on in another process: poll for the file.
    run([EDGE, "--headless=new", "--disable-gpu", f"--user-data-dir={prof}", f"--window-size={W},{H}", f"--screenshot={png}", url], 30)
    t0 = time.time()
    while not png.exists() and time.time() - t0 < 60:
        time.sleep(0.3)
    ok = png.exists()
    if ok:
        time.sleep(0.7)  # let the file be finished and the browser exit by itself
        shutil.copyfile(png, final)
    left = sweep(prof.name)  # kills only what is still running with this profile
    for _ in range(10):
        try:
            shutil.rmtree(prof)
            break
        except OSError:
            time.sleep(0.5)
    png.unlink(missing_ok=True)
    return name, {"ok": ok, "stragglers_killed": left}


def compose_one(name):
    from PIL import Image, ImageDraw
    a, b = OUT / f"{name}-ours.png", OUT / f"{name}-edge.png"
    if not a.exists() and not b.exists():
        return name, False
    scale = 0.5
    panels = []
    for p in (a, b):
        if p.exists():
            im = Image.open(p).convert("RGB")
            panels.append(im.resize((int(im.width * scale), int(im.height * scale)), Image.LANCZOS))
        else:
            panels.append(Image.new("RGB", (int(W * scale), int(H * scale)), (200, 80, 80)))
    h = max(p.height for p in panels)
    sheet = Image.new("RGB", (panels[0].width + panels[1].width + 12, h + 18), (90, 90, 90))
    sheet.paste(panels[0], (0, 18))
    sheet.paste(panels[1], (panels[0].width + 12, 18))
    d = ImageDraw.Draw(sheet)
    d.text((4, 3), f"{name}: qinglan (left) | Edge JS-off (right)", fill=(255, 255, 255))
    sheet.save(OUT / f"{name}.png")
    return name, metrics(a, b)


def metrics(a: Path, b: Path):
    """Rough numbers for triage, not a verdict: share of non-background pixels and the lowest non-background row, per picture,
    and the mean absolute difference of the two pictures reduced to 160x300 grey."""
    from PIL import Image, ImageChops, ImageStat
    out = {}
    ims = {}
    for key, p in (("ours", a), ("edge", b)):
        if not p.exists():
            continue
        im = Image.open(p).convert("L")
        ims[key] = im
        bg = im.getpixel((0, 0))
        mask = im.point(lambda v, bg=bg: 255 if abs(v - bg) > 24 else 0)
        small = mask.resize((160, 300), Image.BOX)
        bbox = mask.getbbox()
        out[key + "_ink"] = round(ImageStat.Stat(mask).mean[0] / 255, 4)
        out[key + "_last_row"] = round(bbox[3] / im.height, 3) if bbox else 0.0
    if len(ims) == 2:
        x, y = (i.resize((160, 300), Image.BOX) for i in (ims["ours"], ims["edge"]))
        out["diff"] = round(ImageStat.Stat(ImageChops.difference(x, y)).mean[0], 1)
    return out


# ---- text check -------------------------------------------------------------------------------------

def unescape(s: str) -> str:
    out, i = [], 0
    while i < len(s):
        c = s[i]
        if c == "\\" and i + 1 < len(s):
            n = s[i + 1]
            if n == "u" and s[i + 2 : i + 3] == "{":
                j = s.find("}", i)
                out.append(chr(int(s[i + 3 : j], 16)))
                i = j + 1
                continue
            out.append({"n": "\n", "t": "\t", "r": "\r", '"': '"', "\\": "\\", "'": "'"}.get(n, n))
            i += 2
            continue
        out.append(c)
        i += 1
    return "".join(out)


def layout_text(dump: str) -> str:
    parts = []
    for line in dump.splitlines():
        m = re.match(r'^\s*text "(.*)" \(', line)
        if m:
            parts.append(unescape(m.group(1)))
    return "".join(parts)


SKIP = {"script", "style", "template", "head"}


def style_displays(dump: str) -> list:
    """The `display` of every element of the `style` dump, in tree order (an element line has no ': ' in it)."""
    out, want = [], False
    for line in dump.splitlines():
        t = line.strip()
        if not t:
            continue
        if ": " not in t:
            out.append(None)
            want = True
        elif want and t.startswith("display: "):
            out[-1] = t[9:]
            want = False
        else:
            want = False
    return out


def dom_text(dump: str, displays) -> tuple:
    """Text of the DOM that is rendered: not under script/style/template/head or a display:none element.
    Returns (text, ok) where ok says the style dump lined up with the DOM dump element for element."""
    parts, stack, buf, n_el = [], [], None, 0
    elems = []
    for line in dump.split("\n"):
        if buf is not None:
            buf.append(line)
            joined = "\n".join(buf)
            if len(joined) >= 2 and joined.endswith('"'):
                elems.append(("t", [x for x in stack], joined[1:-1]))
                buf = None
            continue
        if not line.startswith("| "):
            continue
        body = line[2:]
        depth = (len(body) - len(body.lstrip(" "))) // 2
        s = body.lstrip(" ")
        if s.startswith("<") and not s.startswith("<!"):
            while stack and stack[-1][1] >= depth:
                stack.pop()
            name = s[1:].split(">")[0]
            # The style dump has no elements inside <template> content (svg and math ones it does have).
            counted = not any(t == "template" for t, _, _ in stack)
            stack.append((name.split(" ")[-1], depth, n_el if counted else -1))
            n_el += 1 if counted else 0
        elif s.startswith('"'):
            while stack and stack[-1][1] >= depth:
                stack.pop()
            if len(s) >= 2 and s.endswith('"'):
                elems.append(("t", [x for x in stack], s[1:-1]))
            else:
                buf = [s]
    ok = len(displays) == n_el
    for _, anc, text in elems:
        if any(tag in SKIP for tag, _, _ in anc):
            continue
        if ok and any(i >= 0 and displays[i] == "none" for _, _, i in anc):
            continue
        parts.append(text)
    return "".join(parts), ok


def strip_ws(s: str) -> str:
    return re.sub(r"\s+", "", s)


def align(a: str, b: str, k: int = 12):
    """Walk `a` in chunks of k chars and look each one up in `b` just after the last hit. Returns the number of chars
    matched in order, found elsewhere in b (order problem), not in b at all, and a sample of the first bad chunks."""
    pos = matched = late = absent = 0
    samples = []
    for i in range(0, len(a), k):
        c = a[i : i + k]
        j = b.find(c, pos, pos + 4000 + k)  # a bounded window: a repeated phrase far ahead must not drag the pointer
        if j >= 0:
            pos, matched = j + len(c), matched + len(c)
        else:
            if b.find(c) >= 0:
                late += len(c)
            else:
                absent += len(c)
            if len(samples) < 3:
                samples.append(c)
    return matched, late, absent, samples


def check_one(name):
    u, st = first_url(name)
    if not u:
        return name, None
    d = str(CORPUS / name)
    rc1, lay, lerr, t1 = run([str(CLI), "layout", u, "--replay", d, "--width", str(W), "--height", str(H)], 120)
    rc2, dom, derr, t2 = run([str(CLI), "dom", u, "--replay", d], 120)
    rc3, sty, serr, t3 = run([str(CLI), "style", u, "--replay", d, "--width", str(W), "--height", str(H)], 120)
    r = {"layout_rc": rc1, "dom_rc": rc2, "timeout": t1 or t2 or t3}
    m = re.search(r"\.notdef glyphs after fallback: (\d+)(?: \(first characters: ([^)]*)\))?", lerr)
    r["notdef"] = int(m.group(1)) if m else 0
    r["notdef_chars"] = (m.group(2) or "") if m else ""
    m = re.search(r"note: (\d+) images broken, (\d+) not arrived", lerr)
    r["img_broken"], r["img_pending"] = (int(m.group(1)), int(m.group(2))) if m else (0, 0)
    r["stderr_notes"] = [l for l in lerr.splitlines() if l.startswith(("note:", "warning:", "error:"))][:6]
    dtext, aligned = dom_text(dom, style_displays(sty))
    L, D = strip_ws(layout_text(lay)), strip_ws(dtext)
    r["layout_chars"], r["dom_chars"], r["style_aligned"] = len(L), len(D), aligned
    r["fffd_layout"], r["fffd_dom"] = L.count("�"), D.count("�")
    # laid out but not in the DOM text / in a different order (extra), and in the DOM text but not laid out (dropped)
    mt, late, absent, samp = align(L, D)
    r["extra_late"], r["extra_absent"], r["extra_sample"] = late, absent, samp
    mt2, late2, absent2, samp2 = align(D, L)
    r["dropped_late"], r["dropped_absent"], r["dropped_sample"] = late2, absent2, samp2
    r["coverage"] = round(mt2 / len(D), 4) if D else 1.0
    return name, r


def bench_one(name):
    u, st = first_url(name)
    if not u:
        return name, None
    runs = []
    for _ in range(3):
        rc, out, err, to = run([str(CLI), "bench", u, "--replay", str(CORPUS / name), "--width", "1920", "--height", "1080"], 150)
        kv = dict(re.findall(r"^([a-z0-9-]+): (.*)$", out, re.M))
        runs.append({"rc": rc, "timeout": to, **{k: kv.get(k) for k in ("first-screen-ms", "full-layout-ms", "private-bytes-after-scroll", "peak-private-bytes", "scroll-median-ms", "scroll-p95-ms")}})
    def med(key):
        vals = [float(r[key]) for r in runs if r.get(key) not in (None, "n/a")]
        return statistics.median(vals) if vals else None
    return name, {"ok": all(r["rc"] == 0 for r in runs), "first_screen_ms": med("first-screen-ms"), "full_layout_ms": med("full-layout-ms"),
                  "private_bytes": med("private-bytes-after-scroll"), "peak_bytes": med("peak-private-bytes"),
                  "scroll_median_ms": med("scroll-median-ms"), "scroll_p95_ms": med("scroll-p95-ms"), "runs": runs}


EXE = ROOT / "target" / "release" / "qinglan.exe"


def winbench_one(name):
    """The real exe's own --bench (a window opens for a few seconds), once per page, 1920x1080 at dpr 1."""
    u, st = first_url(name)
    if not u:
        return name, None
    rc, out, err, to = run([str(EXE), "--bench", u, "--replay", str(CORPUS / name), "--width", "1920", "--height", "1080", "--dpr", "1"], 300)
    kv = dict(re.findall(r"^([a-z0-9-]+): (.*)$", out, re.M))
    f = lambda k: float(kv[k]) if k in kv and kv[k] != "n/a" else None
    return name, {"rc": rc, "timeout": to, "first_screen_ms": f("first-screen-ms"), "full_layout_ms": f("full-layout-ms"), "private_bytes": f("private-bytes-after-scroll"),
                  "peak_bytes": f("peak-private-bytes"), "scroll_median_ms": f("scroll-median-ms"), "scroll_p95_ms": f("scroll-p95-ms"), "error": kv.get("bench-error")}


def pmap(fn, names, workers):
    with cf.ThreadPoolExecutor(workers) as ex:
        return list(ex.map(fn, names))


def main():
    stage = sys.argv[1]
    keep_edge = "--keep-edge" in sys.argv
    names = [a for a in sys.argv[2:] if not a.startswith("--")] or [p[0] for p in PAGES]
    OUT.mkdir(parents=True, exist_ok=True)
    res = load()
    if stage == "record":
        for n, msg in pmap(record_one, names, 4):
            print(n, msg, flush=True)
    elif stage == "render":
        for n, r in pmap(render_one, names, 4):
            res.setdefault(n, {})["render"] = r
            print(n, r and (r["rc"], r["timeout"]), flush=True)
    elif stage == "edge" and keep_edge:
        for n in names:
            e = OUT / f"{n}-edge.png"
            print(n, "kept" if e.exists() else "MISSING", flush=True)
    elif stage == "edge":
        for n, r in pmap(edge_one, names, 3):
            res.setdefault(n, {})["edge"] = r
            print(n, r, flush=True)
    elif stage == "compose":
        for n, m in pmap(compose_one, names, 4):
            res.setdefault(n, {})["images"] = m
            print(n, m)
    elif stage == "check":
        for n, r in pmap(check_one, names, 4):
            res.setdefault(n, {})["check"] = r
            print(n, r and {k: r[k] for k in ("coverage", "style_aligned", "extra_late", "extra_absent", "dropped_late", "dropped_absent", "notdef", "fffd_layout", "img_broken")}, flush=True)
    elif stage == "bench":
        for n in names:
            n, r = bench_one(n)
            res.setdefault(n, {})["bench"] = r
            print(n, r and {k: r[k] for k in ("ok", "first_screen_ms", "full_layout_ms", "private_bytes")}, flush=True)
    elif stage == "winbench":
        for n in names:
            n, r = winbench_one(n)
            res.setdefault(n, {})["winbench"] = r
            print(n, r and {k: r[k] for k in ("rc", "first_screen_ms", "full_layout_ms", "private_bytes", "scroll_median_ms")}, flush=True)
    elif stage == "pages":
        lines = ["# 真实网页库", "", "录制命令：`qinglan-cli record <网址> tests/corpus/local/<名字> --width 1280 --height 2400`。录制文件不提交（有版权），",
                 "本表记下网址、类别、录制日期。每页都没有登录，不含个人信息；录制不带 Cookie。`zhwiki-rec` 是第 1c-2 步起的性能页（同一个页面，旧录制）。", "",
                 "| 名字 | 网址 | 类别 | 录制日期 | 状态 |", "| --- | --- | --- | --- | --- |"]
        for n, cat, url in PAGES:
            idx = CORPUS / n / "index.txt"
            if not idx.exists():
                continue
            date = datetime.date.fromtimestamp(idx.stat().st_mtime).isoformat()
            _, st = first_url(n)
            lines.append(f"| {n} | {url} | {cat} | {date} | HTTP {st} |")
        print("\n".join(lines))
    save(res)


if __name__ == "__main__":
    main()
