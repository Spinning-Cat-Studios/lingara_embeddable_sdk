//! Dotted JSON paths, as every refusal and warning names them:
//! `checkpoints[1].edges[0].predicate`. The document itself is `""`.

/// `base.key`, or `key` at the top.
pub(crate) fn member(base: &str, key: &str) -> String {
    if base.is_empty() { key.to_string() } else { format!("{base}.{key}") }
}

/// `base[i]`.
pub(crate) fn index(base: &str, i: usize) -> String {
    format!("{base}[{i}]")
}

/// `checkpoints[i].edges[j]`.
pub(crate) fn edge(checkpoint: usize, edge: usize) -> String {
    format!("checkpoints[{checkpoint}].edges[{edge}]")
}
