use std::path::Path;

/// Sanitize a group_id for use with FalkorDB/RediSearch.
///
/// RediSearch TEXT fields tokenize on hyphens, so `chromatic-poe` becomes
/// two tokens `[chromatic, poe]` which breaks `@group_id` filters.
/// Underscores are the one punctuation character RediSearch preserves.
pub fn sanitize_group_id(id: String) -> String {
    id.replace('-', "_")
}

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
                return current
                    .file_name()
                    .map(|n| sanitize_group_id(n.to_string_lossy().to_string()));
            }
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => break,
        }
    }

    dir.file_name()
        .map(|n| sanitize_group_id(n.to_string_lossy().to_string()))
}
