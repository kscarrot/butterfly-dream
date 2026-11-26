//! 将本目录 `*.rymd` 批量编译为同名 `*.html`。
//!
//! 外壳模板：同目录 [`preview.html`]，占位符 `{{TITLE}}` / `{{BODY}}`。

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use doc_core::html_escape;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

fn preview_template() -> &'static str {
    static TEMPLATE: OnceLock<String> = OnceLock::new();
    TEMPLATE.get_or_init(|| {
        let path = examples_dir().join("preview.html");
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    })
}

fn wrap_html(title: &str, body_html: &str) -> String {
    preview_template()
        .replace("{{TITLE}}", &html_escape(title))
        .replace("{{BODY}}", body_html)
}

fn main() {
    let dir = examples_dir();
    let entries = fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));

    let mut count = 0usize;
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("rymd"))
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
