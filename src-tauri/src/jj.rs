use crate::vcs::{Bookmark, Change};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

fn parse_changes(output: &str) -> Vec<Change> {
    // JJ templates can emit literal "\n" markers instead of real line breaks.
    output
        .split("\\n")
        .flat_map(|chunk| chunk.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let id = parts.next()?.trim().to_string();
            if id.is_empty() {
                return None;
            }
            let description = parts.next().unwrap_or("(working copy)").trim().to_string();
            let timestamp = parts.next().unwrap_or("").trim().to_string();
            Some(Change {
                id,
                description,
                timestamp,
            })
        })
        .collect()
}

fn fetch_changes(repo: &Path, revset: &str) -> Vec<Change> {
    let output = Command::new("jj")
        .args([
            "--no-pager",
            "log",
            "--no-graph",
            "-r",
            revset,
            "-T",
            "commit_id.short(8) ++ \"\\t\" ++ description.first_line() ++ \"\\t\" ++ committer.timestamp().format(\"%Y-%m-%d %H:%M\") ++ \"\\n\"",
        ])
        .current_dir(repo)
        .output();

    match output {
        Ok(o) if o.status.success() => parse_changes(&String::from_utf8_lossy(&o.stdout)),
        _ => vec![],
    }
}

/// List recent changes from jj log.
pub fn changes(repo: &Path, limit: usize, before_id: Option<String>) -> Vec<Change> {
    // Basic check to see if we can even run jj
    let check = Command::new("jj")
        .args(["--no-pager", "log", "-r", "root()", "-T", "commit_id"])
        .current_dir(repo)
        .output();
    if check.is_err() || !check.unwrap().status.success() {
        println!("[BACKEND] jj check failed, returning empty history");
        return vec![];
    }

    // Fetch full linear ancestry once and paginate in Rust. This avoids overlap
    // or cursor issues from revset arithmetic across different JJ versions.
    let all = fetch_changes(repo, "(ancestors(@) ~ root())");
    if all.is_empty() {
        println!("[BACKEND] History fetch failed or returned nothing");
        return vec![];
    }

    let page = match before_id.as_ref() {
        None => all.into_iter().take(limit).collect::<Vec<_>>(),
        Some(id) if id.is_empty() => all.into_iter().take(limit).collect::<Vec<_>>(),
        Some(id) => {
            let cursor = all.iter().position(|c| c.id == *id || c.id.starts_with(id));
            match cursor {
                Some(index) => all
                    .into_iter()
                    .skip(index + 1)
                    .take(limit)
                    .collect::<Vec<_>>(),
                None => vec![],
            }
        }
    };

    println!(
        "[BACKEND] Returning {} changes (limit: {}, before_id: {:?})",
        page.len(),
        limit,
        before_id
    );
    page
}

/// List all bookmarks (branches) in the repo.
pub fn bookmarks(repo: &Path) -> Vec<Bookmark> {
    let output = Command::new("jj")
        .args(["--no-pager", "bookmark", "list"])
        .current_dir(repo)
        .output();

    let output = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return vec![],
    };

    output
        .lines()
        .filter_map(|line| {
            // Default format: "name: id" or "name (type): id"
            let mut parts = line.split(':');
            let name_part = parts.next()?.trim();
            let id = parts.next()?.trim().to_string();

            // Clean name (remove "(local)" etc)
            let name = name_part.split(' ').next()?.to_string();

            if name.is_empty() {
                return None;
            }
            Some(Bookmark {
                name,
                id: id.chars().take(8).collect(),
            })
        })
        .collect()
}

/// Get files changed since a revset, with their status.
pub fn changed_files(repo: &Path, revset: &str) -> HashMap<String, String> {
    let mut args = vec![
        "--no-pager".to_string(),
        "diff".to_string(),
        "--summary".to_string(),
    ];

    if revset == "00000000" || revset == "zzzzzzzz" || revset == "root()" {
        return HashMap::new();
    }

    // Use -r to show changes introduced BY the revision
    args.extend(["-r".to_string(), revset.to_string()]);

    let output = Command::new("jj").args(&args).current_dir(repo).output();

    let output = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return HashMap::new(),
    };

    output
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, ' ');
            let status = parts.next()?;
            let path = parts.next()?.trim().to_string();
            let status = match status {
                "A" => "added",
                "M" => "modified",
                "D" => "deleted",
                _ => return None,
            };
            Some((path, status.to_string()))
        })
        .collect()
}

/// Get the diff for a single file, optionally since a revset.
pub fn file_diff(repo: &Path, file: &str, since: Option<&str>) -> String {
    let mut args = vec![
        "--no-pager".to_string(),
        "diff".to_string(),
        "--git".to_string(),
    ];
    if let Some(s) = since {
        if s == "00000000" || s == "zzzzzzzz" || s == "root()" {
            // Root has no diff
            return String::new();
        }
        args.extend(["-r".to_string(), s.to_string()]);
    }
    args.extend(["--".to_string(), file.to_string()]);

    let output = Command::new("jj").args(&args).current_dir(repo).output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => String::new(),
    }
}
