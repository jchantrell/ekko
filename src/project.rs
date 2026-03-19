use std::path::Path;

/// Reserved group_id for memories that aren't scoped to any project.
pub const GLOBAL_GROUP_ID: &str = "_global";

/// Detect project name from a directory path.
///
/// Walks up from `dir` looking for VCS roots (.git) or workspace markers
/// (Cargo.toml, package.json, etc). Returns the directory name of the
/// first match, or the leaf directory name as fallback.
pub fn detect_group_id(dir: &Path) -> Option<String> {
    let markers = [
        ".git",
        "Cargo.toml",
        "package.json",
        "go.mod",
        "pyproject.toml",
        "Makefile",
        ".project",
    ];

    let mut current = dir;
    loop {
        for marker in &markers {
            if current.join(marker).exists() {
                return current.file_name().map(|n| n.to_string_lossy().to_string());
            }
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => break,
        }
    }

    dir.file_name().map(|n| n.to_string_lossy().to_string())
}

/// Build group_ids for read operations: project scope + global.
///
/// Always includes `_global` so global memories surface alongside
/// project-scoped results.
pub fn read_group_ids(project: Option<String>) -> Vec<String> {
    let mut ids = vec![GLOBAL_GROUP_ID.to_string()];
    if let Some(g) = project
        && g != GLOBAL_GROUP_ID
    {
        ids.push(g);
    }
    ids
}
