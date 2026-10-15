# Prepares the performance test page (tests/corpus/local/zhwiki/, git-ignored: Wikipedia text is copyrighted).
# One download of one article and its three style sheets, then the links are rewritten to the local files.
# Run: python -I tools/prepare_zhwiki.py
import io, os, re, urllib.parse, urllib.request

UA = "qinglan-dev/0.1 (performance corpus, single fetch)"
OUT = os.path.join("tests", "corpus", "local", "zhwiki")
os.makedirs(OUT, exist_ok=True)

def get(url):
    return urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": UA}), timeout=60).read()

title = "中华人民共和国"
page = get("https://zh.wikipedia.org/wiki/" + urllib.parse.quote(title))
html = page.decode("utf-8")
hrefs = re.findall(r'<link rel="stylesheet" href="(/w/load\.php[^"]*)"', html)
for i, h in enumerate(hrefs):
    css = get("https://zh.wikipedia.org" + h.replace("&amp;", "&"))
    open(os.path.join(OUT, "css%d.css" % i), "wb").write(css)
    html = html.replace('<link rel="stylesheet" href="%s"' % h, '<link rel="stylesheet" href="css%d.css"' % i)
io.open(os.path.join(OUT, "page-local.html"), "w", encoding="utf-8", newline="").write(html)
print(len(page), "bytes,", len(hrefs), "style sheets")
