import json, re, sys, pathlib

src = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
for name in sys.argv[3:]:
    js = (src / f"{name}.js").read_text(encoding="utf-8")
    body = js[js.index("createLucideIcon(") :]
    nodes = re.findall(r'\[\s*"(\w+)",\s*(\{.*?\})\s*\]', body, re.S)
    parts = []
    for tag, attrs in nodes:
        attrs = re.sub(r'(\w+):', r'"\1":', attrs)
        values = {k: v for k, v in json.loads(attrs).items() if k != "key"}
        parts.append(f"<{tag} " + " ".join(f'{k}="{v}"' for k, v in values.items()) + "/>")
    svg = '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">' + "".join(parts) + "</svg>\n"
    (out / f"{name}.svg").write_text(svg, encoding="utf-8")
    print(name, len(parts))
