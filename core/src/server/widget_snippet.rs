//! The widget snippet (ADR 1.10.26af D7, ES4): the `<lingara-embed>`
//! element and its two module scripts, rendered identically by all eight
//! server kits. The markup is the widget's own (K6); the token never appears
//! in it, only the endpoint the element asks at run time.

use super::snippet_rules as rule;

/// The script-tag build of this release's element.
pub const DEFAULT_ELEMENT_SRC: &str =
    concat!("https://cdn.jsdelivr.net/npm/@lingara/embed@", env!("CARGO_PKG_VERSION"), "/dist/lingara-embed.min.js");

/// What to render. `plan_id` and `token_endpoint` are required; every other
/// field is written only when set.
#[derive(Clone, Debug, Default)]
pub struct SnippetOptions {
    /// The lesson plan your server made for this player, a UUID.
    pub plan_id: String,
    /// Your token route: a path (`/lingara/token`) or an `https://` URL.
    /// The element `POST`s to it with the page's cookies and reads
    /// `token_handoff`'s body.
    pub token_endpoint: String,
    /// Default `practice`.
    pub surface: Option<String>,
    /// `light`, `dark` or `auto`.
    pub theme: Option<String>,
    pub locale: Option<String>,
    pub lingara_origin: Option<String>,
    pub title: Option<String>,
    /// Default `lingara-embed`; two widgets on one page need two ids.
    pub id: Option<String>,
    /// Your page's CSP nonce, written on both scripts.
    pub nonce: Option<String>,
    /// Default [`DEFAULT_ELEMENT_SRC`].
    pub element_src: Option<String>,
}

impl SnippetOptions {
    pub fn new(plan_id: impl Into<String>, token_endpoint: impl Into<String>) -> Self {
        Self { plan_id: plan_id.into(), token_endpoint: token_endpoint.into(), ..Self::default() }
    }
}

/// The first option refused, by its contract name (`planId`, `surface`, …).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the widget snippet's {field} is not valid")]
pub struct SnippetError {
    pub field: &'static str,
}

/// The three lines to paste into your page, with no trailing newline.
pub fn widget_snippet(o: &SnippetOptions) -> Result<String, SnippetError> {
    let surface = o.surface.as_deref().unwrap_or("practice");
    let id = o.id.as_deref().unwrap_or("lingara-embed");
    let src = o.element_src.as_deref().unwrap_or(DEFAULT_ELEMENT_SRC);
    check(o, surface, id)?;
    let mut element = format!("<lingara-embed id=\"{}\" surface=\"{}\" plan-id=\"{}\"", attr(id), attr(surface), attr(&o.plan_id));
    let optional = [("theme", &o.theme), ("locale", &o.locale), ("lingara-origin", &o.lingara_origin), ("title", &o.title)];
    for (name, value) in optional {
        if let Some(value) = value {
            element.push_str(&format!(" {name}=\"{}\"", attr(value)));
        }
    }
    let nonce = o.nonce.as_deref().map(|n| format!(" nonce=\"{}\"", attr(n))).unwrap_or_default();
    Ok(format!(
        "{element}></lingara-embed>\n<script type=\"module\"{nonce} src=\"{}\"></script>\n<script type=\"module\"{nonce}>document.getElementById({}).tokenProvider = async () => (await fetch({}, {{ method: \"POST\", credentials: \"same-origin\" }})).json();</script>",
        attr(src),
        script_string(id),
        script_string(&o.token_endpoint),
    ))
}

/// Each rule in the contract's order; the first failure names its option.
fn check(o: &SnippetOptions, surface: &str, id: &str) -> Result<(), SnippetError> {
    let opt = |v: &Option<String>, ok: fn(&str) -> bool| v.as_deref().is_none_or(ok);
    let rules = [
        ("planId", rule::plan_id(&o.plan_id)),
        ("surface", rule::surface(surface)),
        ("theme", opt(&o.theme, |t| matches!(t, "light" | "dark" | "auto"))),
        ("locale", opt(&o.locale, rule::locale)),
        ("lingaraOrigin", opt(&o.lingara_origin, rule::lingara_origin)),
        ("title", opt(&o.title, rule::title)),
        ("id", rule::id(id)),
        ("tokenEndpoint", rule::token_endpoint(&o.token_endpoint)),
        ("nonce", opt(&o.nonce, rule::nonce)),
        ("elementSrc", opt(&o.element_src, rule::element_src)),
    ];
    match rules.into_iter().find(|(_, ok)| !ok) {
        Some((field, _)) => Err(SnippetError { field }),
        None => Ok(()),
    }
}

/// An attribute value: the five characters, and nothing else.
fn attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// A JSON string literal that can never close the script or open a comment.
fn script_string(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
