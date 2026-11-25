//! 将 `examples/*.jmd` 批量编译为同名 `examples/<name>.html`。

use std::fs;
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn wrap_html(title: &str, body_html: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <style>
    :root {{
      color-scheme: light dark;
      --text: CanvasText;
      --muted: color-mix(in srgb, CanvasText 55%, Canvas);
      --person: #8b1a1a;
      --place: #1a5f2a;
      --time: #6b4a1a;
      --office: #2a2a7a;
      --rule: color-mix(in srgb, CanvasText 18%, transparent);
    }}
    @media (prefers-color-scheme: dark) {{
      :root {{
        --person: #e8a0a0;
        --place: #8fd4a0;
        --time: #e0c08a;
        --office: #a0a0e8;
      }}
    }}
    html, body {{
      height: 100%;
    }}
    body {{
      box-sizing: border-box;
      margin: 0;
      padding: 2.5rem 2rem 2rem 3rem;
      writing-mode: vertical-rl;
      text-orientation: mixed;
      overflow-x: auto;
      overflow-y: hidden;
      font: 20px/1.95 "Songti SC", "Noto Serif CJK SC", "Source Han Serif SC", "SimSun", serif;
      color: var(--text);
      letter-spacing: 0.06em;
    }}
    h1, h2 {{
      font-weight: 600;
      line-height: 1.5;
      margin: 0 1.25em 0 0;
    }}
    p {{
      margin: 0 1.1em 0 0;
      max-height: calc(100vh - 5rem);
    }}
    .jmd-person {{
      color: var(--person);
      text-decoration: none;
      border-inline-end: 1px dotted currentColor;
      padding-inline-end: 0.05em;
    }}
    .jmd-place {{ color: var(--place); }}
    .jmd-time {{ color: var(--time); }}
    .jmd-office {{ color: var(--office); }}
    .jmd-style.note {{ font-size: 0.92em; color: var(--muted); }}
    .jmd-revision a {{
      color: var(--muted);
      font-size: 0.72em;
      writing-mode: horizontal-tb;
      display: inline-block;
      transform: translateY(0.1em);
    }}
    .footnotes {{
      margin-inline-start: 2rem;
      padding-inline-start: 1rem;
      border-inline-start: 1px solid var(--rule);
      font-size: 0.9em;
      color: var(--muted);
      max-height: calc(100vh - 5rem);
    }}
    .footnotes ol {{ padding: 0; margin: 0; }}
    .sr-only {{ position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0,0,0,0); }}
    code {{
      font-family: ui-monospace, monospace;
      font-size: 0.86em;
      writing-mode: horizontal-tb;
    }}
    a[data-footnote-ref], a[data-footnote-backref] {{
      writing-mode: horizontal-tb;
      display: inline-block;
    }}
  </style>
</head>
<body>
{body}
</body>
</html>
"##,
        title = html_escape(title),
        body = body_html
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn main() {
    let dir = examples_dir();
    let entries = fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));

    let mut count = 0usize;
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("jmd"))
        .collect();
    paths.sort();

    for jmd in paths {
        let stem = jmd.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
        let src = fs::read_to_string(&jmd).unwrap_or_else(|e| panic!("read {}: {e}", jmd.display()));
        let doc = doc_core::parse(&src);
        let out = doc_core::render(&doc);

        let title = out
            .frontmatter
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(stem);

        let html = wrap_html(title, &out.body_html);
        let dest = dir.join(format!("{stem}.html"));
        fs::write(&dest, html).unwrap_or_else(|e| panic!("write {}: {e}", dest.display()));
        println!("wrote {}", dest.display());
        count += 1;
    }

    println!("done: {count} file(s)");
}
