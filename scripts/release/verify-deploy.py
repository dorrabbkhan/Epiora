#!/usr/bin/env python3
"""Post-deploy verification for the Epiora web build (docs/releasing.md,
Phase 5). Stdlib only. Read-only: GET/HEAD requests only, nothing is
deployed, uploaded or configured.

    scripts/release/verify-deploy.py https://epiora-dev.netlify.app
    scripts/release/verify-deploy.py --self-test

Checks: HTTP -> HTTPS redirect; / and /index.html serve HTML; the SPA
fallback (/p/does-not-exist -> 200 HTML); every app/public/_headers rule
(CSP exact match, nosniff, no-referrer, Cache-Control for /index.html,
/sw.js and /assets/*, manifest content type); /sw.js and
/manifest.webmanifest exist; every /assets/* path referenced from
index.html and the JS answers 200 with a non-HTML content type
(.js a JavaScript type, .css text/css, .wasm application/wasm — a
missing asset would come back as the SPA fallback's 200 text/html); no
JS contains the VITE_TEST_HOOKS identifier and some JS contains the
test-hook canary "test hooks are disabled".

Prints PASS/FAIL (and SKIP) per check; exit 1 on any FAIL.
"""

import http.server
import os
import re
import sys
import tempfile
import threading
import urllib.error
import urllib.parse
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
HEADERS_FILE = os.path.join(ROOT, "app", "public", "_headers")
CANARY = "test hooks are disabled"
FORBIDDEN = "VITE_TEST_HOOKS"
TIMEOUT = 30
MAX_ASSETS = 500
ASSET_RE = re.compile(r"assets/[A-Za-z0-9._~-]+\.(?:js|css|wasm|svg|png|webmanifest|ico)")
REF_RE = re.compile(r"""(?:src|href)=["'](/[^"']+)["']""")

CT = {
    ".html": "text/html", ".js": "text/javascript", ".css": "text/css",
    ".wasm": "application/wasm", ".webmanifest": "application/manifest+json",
    ".svg": "image/svg+xml", ".png": "image/png", ".ico": "image/x-icon",
}

# Content type a crawled /assets/* file must be served with; anything
# not listed just has to not be text/html (the SPA fallback's type).
ASSET_CT = {".js": "javascript", ".css": "text/css", ".wasm": "application/wasm"}


def load_rules(path):
    """Parse a Netlify _headers file into [(pattern, [(key, value), ...])]."""
    rules, current = [], None
    for raw in open(path, encoding="utf-8"):
        line = raw.rstrip("\n")
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if line.startswith("/"):
            current = (line.strip(), [])
            rules.append(current)
        elif current and ":" in line:
            k, v = line.strip().split(":", 1)
            current[1].append((k.strip(), v.strip()))
    return rules


def rule_matches(pattern, path):
    if pattern.endswith("/*"):
        return path.startswith(pattern[:-1])
    return pattern == path


def expected_headers(path, rules):
    """Netlify applies every matching block in order; later keys win."""
    exp = {}
    for pattern, pairs in rules:
        if rule_matches(pattern, path):
            for k, v in pairs:
                exp[k.lower()] = (k, v)
    return exp


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


NO_REDIRECT_OPENER = urllib.request.build_opener(NoRedirect)


def fetch(url, redirects=True):
    """-> (status, headers-as-dict-of-lowercase, body bytes); status -1 on error."""
    req = urllib.request.Request(url, headers={"User-Agent": "epiora-verify-deploy"})
    opener = urllib.request.build_opener() if redirects else NO_REDIRECT_OPENER
    try:
        with opener.open(req, timeout=TIMEOUT) as r:
            return r.status, {k.lower(): v for k, v in r.headers.items()}, r.read()
    except urllib.error.HTTPError as e:
        body = b""
        try:
            body = e.read()
        except Exception:
            pass
        return e.code, {k.lower(): v for k, v in e.headers.items()}, body
    except Exception as e:
        return -1, {"x-error": str(e)}, b""


def is_loopback(host):
    return host in ("localhost", "127.0.0.1", "::1", "[::1]")


def run_checks(base_url, rules):
    """Returns (rows, fails): rows = (name, status, detail)."""
    rows = []

    def check(name, ok, detail=""):
        rows.append((name, "PASS" if ok else "FAIL", detail))
        return ok

    def skip(name, why):
        rows.append((name, "SKIP", why))

    base = base_url.rstrip("/")
    parts = urllib.parse.urlsplit(base)
    host = parts.hostname or ""
    fetched = {}

    def get(path):
        if path not in fetched:
            fetched[path] = fetch(base + path)
        return fetched[path]

    # 1. http -> https redirect
    if is_loopback(host) or parts.scheme == "http" and is_loopback(host):
        skip("http→https redirect", "loopback target — exercised on the real deploy only")
    else:
        http_url = "http://" + host + ((":" + str(parts.port)) if parts.port else "") + "/"
        st, hd, _ = fetch(http_url, redirects=False)
        loc = hd.get("location", "")
        check("http→https redirect", st in (301, 302, 307, 308) and loc.startswith("https://"),
              f"{http_url} -> {st} {loc or '(no location)'}")

    # 2. entry pages
    st, hd, body = get("/")
    check("GET /", st == 200 and "text/html" in hd.get("content-type", ""),
          f"status {st}, content-type {hd.get('content-type', '?')}")
    index = body if st == 200 else b""
    st2, hd2, _ = get("/index.html")
    check("GET /index.html", st2 == 200 and "text/html" in hd2.get("content-type", ""),
          f"status {st2}")

    # 3. SPA fallback
    spa = "/p/does-not-exist"
    st3, hd3, _ = get(spa)
    check("SPA fallback " + spa, st3 == 200 and "text/html" in hd3.get("content-type", ""),
          f"status {st3}, content-type {hd3.get('content-type', '?')}")

    # 4. fixed files
    st4, hd4, sw_body = get("/sw.js")
    check("GET /sw.js", st4 == 200 and "javascript" in hd4.get("content-type", ""),
          f"status {st4}, content-type {hd4.get('content-type', '?')}")
    st5, hd5, _ = get("/manifest.webmanifest")
    check("GET /manifest.webmanifest", st5 == 200,
          f"status {st5}, content-type {hd5.get('content-type', '?')}")

    # 5. asset crawl: index.html refs, then js/css -> assets/*
    queue = [p for p in REF_RE.findall(index.decode("utf-8", "replace")) if p.startswith("/")]
    js_bodies, seen = {}, set()
    while queue and len(fetched) < MAX_ASSETS:
        path = queue.pop(0)
        if path in seen:
            continue
        seen.add(path)
        st_a, hd_a, body_a = get(path)
        ctype = hd_a.get("content-type", "?")
        cbase = ctype.split(";")[0].strip().lower()
        # A missing asset is answered by the SPA fallback — 200 text/html
        # — so a bare 200 is not enough: /assets/* must carry a real
        # asset content type.
        ok = st_a == 200
        want = ""
        if path.startswith("/assets/"):
            want = ASSET_CT.get(os.path.splitext(path)[1].lower())
            ok = ok and ((want in cbase) if want else (cbase != "text/html"))
        detail = f"status {st_a}, content-type {ctype}"
        if path.startswith("/assets/"):
            detail += f" (want {want or 'not text/html'})"
        ok = check(f"asset {path}", ok, detail)
        if path.endswith(".wasm"):
            check(f"wasm content-type {path}", cbase == "application/wasm",
                  f"content-type {ctype}")
        if ok:
            text = ""
            if path.endswith((".js", ".css", ".html", ".webmanifest", ".svg")):
                text = body_a.decode("utf-8", "replace")
            for m in ASSET_RE.findall(text):
                queue.append("/" + m.lstrip("/"))
            if path.endswith(".js"):
                js_bodies[path] = text

    # 6. _headers rules on every fetched path
    for path in ["/", "/index.html", spa, "/sw.js", "/manifest.webmanifest"] + sorted(seen):
        exp = expected_headers(path, rules)
        if not exp:
            continue
        st_h, hd_h, _ = get(path)
        if st_h != 200:
            continue  # the fetch check already reports it
        bad = []
        for k, (name, want) in exp.items():
            got = hd_h.get(k)
            if got is None:
                bad.append(f"missing {name}")
            elif k == "cache-control":
                # Netlify re-serializes Cache-Control without the
                # optional whitespace after commas; compare without it.
                if re.sub(r"\s+", "", got) != re.sub(r"\s+", "", want):
                    bad.append(f"{name}: got '{got.strip()}' want '{want}'")
            elif got.strip() != want:
                bad.append(f"{name}: got '{got.strip()}' want '{want}'")
        check(f"headers {path}", not bad, "; ".join(bad) if bad else "all declared headers exact")

    # 7. JS hygiene (the service worker counts too)
    js_bodies["/sw.js"] = sw_body.decode("utf-8", "replace")
    n_bad = [p for p, t in js_bodies.items() if FORBIDDEN in t]
    check(f"no {FORBIDDEN} in JS ({len(js_bodies)} files)", not n_bad,
          "found in " + ", ".join(n_bad) if n_bad else "")
    n_canary = [p for p, t in js_bodies.items() if CANARY in t]
    check(f"canary '{CANARY}' present", bool(n_canary),
          f"in {n_canary[0]}" if n_canary else "in no fetched JS — the deploy may ship a test-hooks build")

    return rows


def report(rows, out):
    fails = sum(1 for _, s, _ in rows if s == "FAIL")
    for name, status, detail in rows:
        line = f"{status:<5} {name}"
        if detail:
            line += f" — {detail}"
        print(line, file=out)
    print(f"--- {len(rows)} checks, {fails} FAIL ---", file=out)
    return fails


# ------------------------- --self-test ---------------------------------

FIXTURE = {
    "index.html": """<!doctype html><html><head>
<link rel="manifest" href="/manifest.webmanifest">
<link rel="stylesheet" href="/assets/index-aaa111.css">
<script type="module" src="/assets/index-aaa111.js"></script>
</head><body>epiora verify-deploy self-test</body></html>""",
    "assets/index-aaa111.css": "body{margin:0}",
    "assets/index-aaa111.js": (
        'import "assets/worker-bbb222.js";\n'
        'const w = new URL("assets/core-ddd444.wasm", import.meta.url);\n'
        '/* test hooks are disabled */\n'
    ),
    "assets/worker-bbb222.js": 'const s = new URL("assets/sqlite-eee555.wasm", import.meta.url);\n',
    "assets/core-ddd444.wasm": b"\x00asm\x01\x00\x00\x00",
    "assets/sqlite-eee555.wasm": b"\x00asm\x01\x00\0\0\0fake",
    "sw.js": "/* workbox stub */\n",
    "manifest.webmanifest": '{"name":"Epiora","short_name":"Epiora"}',
}

# Broken fixture: VITE_TEST_HOOKS in the bundle AND a reference to an
# asset that does not exist — the SPA fallback serves it as 200
# text/html, which the asset check must reject.
BROKEN_JS = 'const flag = "VITE_TEST_HOOKS"; import "assets/worker-bbb222.js";\n' + \
            'import "assets/missing-fff666.js";\n' + \
            'const w = new URL("assets/core-ddd444.wasm", import.meta.url);\n'


def make_fixture(broken):
    d = tempfile.mkdtemp(prefix="epiora-verify-")
    for rel, content in FIXTURE.items():
        if broken and rel == "sw.js":
            continue  # missing sw.js -> the SPA fallback answers HTML
        p = os.path.join(d, *rel.split("/"))
        os.makedirs(os.path.dirname(p), exist_ok=True)
        mode = "wb" if isinstance(content, bytes) else "w"
        with open(p, mode) as f:
            f.write(content)
    if broken:
        with open(os.path.join(d, "assets", "index-aaa111.js"), "w") as f:
            f.write(BROKEN_JS)
    return d


def serve(directory, rules, broken):
    ct = dict(CT)
    if broken:
        ct[".wasm"] = "application/octet-stream"
    drop = {"x-content-type-options", "referrer-policy"} if broken else set()
    csp_override = "default-src *" if broken else None

    class H(http.server.BaseHTTPRequestHandler):
        def log_message(self, *a):
            pass

        def do_GET(self):
            path = urllib.parse.urlsplit(self.path).path
            local = os.path.join(directory, *path.lstrip("/").split("/"))
            if not os.path.isfile(local):
                path, local = "/p/does-not-exist", os.path.join(directory, "index.html")
            ext = os.path.splitext(local)[1]
            body = open(local, "rb").read()
            self.send_response(200)
            self.send_header("Content-Type", ct.get(ext, "application/octet-stream"))
            for k, (name, val) in expected_headers(path, rules).items():
                if k in drop:
                    continue
                if k == "content-security-policy" and csp_override:
                    val = csp_override
                if k == "content-type":
                    continue  # per-extension type already sent
                self.send_header(name, val)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), H)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv


def self_test():
    rules = load_rules(HEADERS_FILE)
    out = sys.stdout
    print("== self-test 1/2: compliant fixture (expect all PASS)", file=out)
    srv = serve(make_fixture(broken=False), rules, broken=False)
    fails_ok = report(run_checks(f"http://127.0.0.1:{srv.server_port}", rules), out)
    srv.shutdown()

    print("\n== self-test 2/2: broken fixture (expect FAILs)", file=out)
    srv = serve(make_fixture(broken=True), rules, broken=True)
    rows = run_checks(f"http://127.0.0.1:{srv.server_port}", rules)
    fails_bad = report(rows, out)
    srv.shutdown()
    got = {n for n, s, _ in rows if s == "FAIL"}
    want = ["GET /sw.js", "headers /", f"no {FORBIDDEN} in JS",
            f"canary '{CANARY}' present",
            "wasm content-type /assets/core-ddd444.wasm",
            "asset /assets/missing-fff666.js"]
    missing = [w for w in want if not any(n.startswith(w) for n in got)]

    print("\n== self-test verdict", file=out)
    ok = fails_ok == 0 and fails_bad > 0 and not missing
    if fails_ok:
        print("FAIL compliant run had FAILs", file=out)
    if missing:
        print(f"FAIL broken run missed expected FAILs: {sorted(missing)}", file=out)
    if not fails_bad:
        print("FAIL broken run produced no FAILs", file=out)
    print("self-test OK" if ok else "self-test FAILED", file=out)
    return 0 if ok else 1


def main(argv):
    if len(argv) == 2 and argv[1] == "--self-test":
        return self_test()
    if len(argv) != 2 or argv[1] in ("-h", "--help") or not argv[1].startswith(("http://", "https://")):
        print(__doc__.strip(), file=sys.stderr)
        return 2
    rules = load_rules(HEADERS_FILE)
    print(f"== verify-deploy {argv[1]}", file=sys.stdout)
    fails = report(run_checks(argv[1], rules), sys.stdout)
    print("OK" if fails == 0 else "FAILURES PRESENT", file=sys.stdout)
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
