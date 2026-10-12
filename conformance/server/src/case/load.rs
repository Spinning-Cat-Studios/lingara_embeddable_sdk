//! Reading case files: one case per `<group>/<slug>.yaml`, id `<group>.<slug>`.

use std::fs;
use std::path::{Path, PathBuf};

use super::{shape_problems, Case};

/// A case read from disk.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub case: Case,
}

/// Every case under each directory, and every problem found on the way: a
/// file that does not deserialise, an id that does not match its path, a
/// duplicated id, or a shape rule broken.
pub fn load_dirs(dirs: &[PathBuf]) -> (Vec<Loaded>, Vec<String>) {
    let mut loaded = Vec::new();
    let mut problems = Vec::new();
    for dir in dirs {
        for path in yaml_files(dir) {
            match read(&path) {
                Ok(case) => {
                    problems.extend(id_problem(dir, &path, &case));
                    problems.extend(shape_problems(&case));
                    loaded.push(Loaded { case });
                }
                Err(e) => problems.push(e),
            }
        }
    }
    let mut ids: Vec<&str> = loaded.iter().map(|l| l.case.id.as_str()).collect();
    ids.sort_unstable();
    for pair in ids.windows(2).filter(|w| w[0] == w[1]) {
        problems.push(format!("{}: the id is used by two cases", pair[0]));
    }
    (loaded, problems)
}

fn read(path: &Path) -> Result<Case, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_yaml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// `<group>/<slug>.yaml` under `dir` must carry id `<group>.<slug>`.
fn id_problem(dir: &Path, path: &Path, case: &Case) -> Option<String> {
    let rel = path.strip_prefix(dir).ok()?;
    let parts: Vec<String> = rel.with_extension("").iter().map(|p| p.to_string_lossy().into_owned()).collect();
    let want = parts.join(".");
    (case.id != want).then(|| format!("{}: id `{}` does not match its path (want `{want}`)", path.display(), case.id))
}

fn yaml_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else { return out };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(yaml_files(&path));
        } else if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
            out.push(path);
        }
    }
    out.sort();
    out
}
