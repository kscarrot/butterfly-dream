//! 将 `doc-core` 诊断映射为 LSP `publishDiagnostics`。

use doc_core::{span_to_range, Diagnostic, Severity};
use tower_lsp::lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, Position, Range,
};

pub fn to_lsp(source: &str, diags: &[Diagnostic]) -> Vec<LspDiagnostic> {
    diags
        .iter()
        .map(|d| {
            let range = match d.span {
                Some(span) => {
                    let (sl, sc, el, ec) = span_to_range(source, span);
                    Range::new(Position::new(sl, sc), Position::new(el, ec))
                }
                None => Range::new(Position::new(0, 0), Position::new(0, 0)),
            };
            LspDiagnostic {
                range,
                severity: Some(match d.severity {
                    Severity::Error => DiagnosticSeverity::ERROR,
                    Severity::Warning => DiagnosticSeverity::WARNING,
                    Severity::Info => DiagnosticSeverity::INFORMATION,
                }),
                code: None,
                code_description: None,
                source: Some("rymd".into()),
                message: d.message.clone(),
                related_information: None,
                tags: None,
                data: None,
            }
        })
        .collect()
}
