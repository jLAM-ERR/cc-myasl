use std::path::Path;
use std::process::Command;

use super::Repo;

/// Counts of file changes in the working tree.
pub struct StatusCounts {
    /// Total changed files (staged + unstaged).
    pub changes: u64,
    /// Files staged for commit (index differs from HEAD).
    pub staged: u64,
    /// Files modified in worktree but not staged.
    pub unstaged: u64,
    /// Files not tracked by git.
    pub untracked: u64,
}

/// Count status entries by shelling out to `git status --porcelain=2`.
///
/// Returns `None` if the command fails or the output cannot be parsed.
/// Shell-out is used here rather than the gix status API because the gix
/// `status` feature pulls in `blob-diff`, `dirwalk`, and `index` — heavy
/// transitive deps that violate the slim-build constraint for v0.1.
pub fn counts(repo: &Repo) -> Option<StatusCounts> {
    let workdir = repo.root()?;
    counts_in_dir(&workdir)
}

/// Run `git status --porcelain=2` in `dir` and parse the output.
fn counts_in_dir(dir: &Path) -> Option<StatusCounts> {
    let out = Command::new("git")
        .args([
            "-C",
            dir.to_str()?,
            "status",
            "--porcelain=2",
            "--untracked-files=all",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(parse_porcelain2(&out.stdout))
}

/// Parse `git status --porcelain=2` output into counts.
///
/// Format reference: https://git-scm.com/docs/git-status#_output_format_version_2
/// - Lines starting with `1 ` or `2 ` are tracked changes (staged or unstaged).
/// - Lines starting with `u ` are unmerged.
/// - Lines starting with `?` are untracked.
/// - Lines starting with `#` are headers.
///
/// For `1 XY …` and `2 XY …`:
///   X = staged status (vs HEAD), Y = worktree status (vs index).
///   '.' means no change for that slot.
fn parse_porcelain2(output: &[u8]) -> StatusCounts {
    let text = std::str::from_utf8(output).unwrap_or("");
    let mut staged = 0u64;
    let mut unstaged = 0u64;
    let mut untracked = 0u64;

    for line in text.lines() {
        if line.starts_with("? ") {
            untracked += 1;
        } else if let Some(rest) = line.strip_prefix("1 ").or_else(|| line.strip_prefix("2 ")) {
            // XY is the first two chars of `rest`.
            let mut chars = rest.chars();
            let x = chars.next().unwrap_or('.');
            let y = chars.next().unwrap_or('.');
            if x != '.' {
                staged += 1;
            }
            if y != '.' {
                unstaged += 1;
            }
        } else if line.starts_with("u ") {
            // Unmerged counts as both staged and unstaged change.
            staged += 1;
            unstaged += 1;
        }
    }

    StatusCounts {
        changes: staged + unstaged + untracked,
        staged,
        unstaged,
        untracked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_porcelain2 unit tests ────────────────────────────────────────

    #[test]
    fn parse_empty_output_all_zeros() {
        let c = parse_porcelain2(b"");
        assert_eq!(c.staged, 0);
        assert_eq!(c.unstaged, 0);
        assert_eq!(c.untracked, 0);
        assert_eq!(c.changes, 0);
    }

    #[test]
    fn parse_untracked_only() {
        let out = b"? new_file.txt\n? another.rs\n";
        let c = parse_porcelain2(out);
        assert_eq!(c.untracked, 2);
        assert_eq!(c.staged, 0);
        assert_eq!(c.unstaged, 0);
        assert_eq!(c.changes, 2);
    }

    #[test]
    fn parse_staged_modified() {
        // X='M' (staged), Y='.' (no worktree change)
        let out = b"1 M. N... 100644 100644 100644 abc def file.txt\n";
        let c = parse_porcelain2(out);
        assert_eq!(c.staged, 1);
        assert_eq!(c.unstaged, 0);
    }

    #[test]
    fn parse_unstaged_modified() {
        // X='.' (nothing staged), Y='M' (worktree modified)
        let out = b"1 .M N... 100644 100644 100644 abc def file.txt\n";
        let c = parse_porcelain2(out);
        assert_eq!(c.staged, 0);
        assert_eq!(c.unstaged, 1);
    }

    #[test]
    fn parse_both_staged_and_unstaged() {
        // X='M' (staged), Y='M' (also modified in worktree)
        let out = b"1 MM N... 100644 100644 100644 abc def file.txt\n";
        let c = parse_porcelain2(out);
        assert_eq!(c.staged, 1);
        assert_eq!(c.unstaged, 1);
        assert_eq!(c.changes, 2);
    }

    #[test]
    fn parse_mixed_entries() {
        let out = b"1 M. N... 100644 100644 100644 a b staged.txt\n\
                    1 .M N... 100644 100644 100644 a b modified.txt\n\
                    ? untracked.txt\n";
        let c = parse_porcelain2(out);
        assert_eq!(c.staged, 1);
        assert_eq!(c.unstaged, 1);
        assert_eq!(c.untracked, 1);
        assert_eq!(c.changes, 3);
    }

    #[test]
    fn parse_header_lines_ignored() {
        let out = b"# branch.oid abc\n# branch.head main\n? foo.txt\n";
        let c = parse_porcelain2(out);
        assert_eq!(c.untracked, 1);
        assert_eq!(c.staged, 0);
    }

    #[test]
    fn parse_unmerged_counts_both() {
        let out = b"u UU N... 0 0 0 0 abc def base merge.txt\n";
        let c = parse_porcelain2(out);
        assert_eq!(c.staged, 1);
        assert_eq!(c.unstaged, 1);
    }
}
