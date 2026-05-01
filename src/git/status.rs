use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use super::Repo;

/// Counts of file changes in the working tree.
pub struct StatusCounts {
    /// Distinct files with any change (modified, staged, unmerged, or untracked).
    /// NOT the sum of staged+unstaged+untracked — a file counted in multiple
    /// categories appears once.
    pub changes: u64,
    /// Files staged for commit (index differs from HEAD).
    pub staged: u64,
    /// Files modified in worktree but not staged.
    pub unstaged: u64,
    /// Files not tracked by git.
    pub untracked: u64,
}

/// Count status entries by shelling out to `git status --porcelain=2 -z`.
///
/// Returns `None` if the command fails or the output cannot be parsed.
/// Shell-out is used here rather than the gix status API because the gix
/// `status` feature pulls in `blob-diff`, `dirwalk`, and `index` — heavy
/// transitive deps that violate the slim-build constraint for v0.1.
pub fn counts(repo: &Repo) -> Option<StatusCounts> {
    let workdir = repo.root()?;
    counts_in_dir(&workdir)
}

/// Run `git status --porcelain=2 -z` in `dir` and parse the output.
fn counts_in_dir(dir: &Path) -> Option<StatusCounts> {
    let out = Command::new("git")
        .args([
            "-C",
            dir.to_str()?,
            "status",
            "--porcelain=2",
            "--untracked-files=all",
            "-z",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(parse_porcelain2_z(&out.stdout))
}

/// Parse `git status --porcelain=2 -z` output into counts.
///
/// With `-z`, entries are NUL-terminated rather than newline-terminated.
/// Filename quoting/escaping is disabled; filenames may contain any byte
/// including newlines.
///
/// For `2` (rename/copy) entries the original path follows as a separate
/// NUL-terminated token immediately after the entry itself.
///
/// Format reference: https://git-scm.com/docs/git-status#_output_format_version_2
fn parse_porcelain2_z(output: &[u8]) -> StatusCounts {
    let mut staged = 0u64;
    let mut unstaged = 0u64;
    let mut untracked = 0u64;
    // Distinct-file set: keyed on the raw bytes of the path portion.
    let mut changed_paths: HashSet<Vec<u8>> = HashSet::new();

    // Split on NUL; each element is one entry (or an orig-path for type-2).
    let mut tokens = output.split(|&b| b == 0).peekable();

    while let Some(entry) = tokens.next() {
        if entry.is_empty() {
            continue;
        }
        if entry.starts_with(b"? ") {
            let path = entry[2..].to_vec();
            untracked += 1;
            changed_paths.insert(path);
        } else if entry.starts_with(b"1 ") || entry.starts_with(b"2 ") {
            let xy = &entry[2..];
            let x = xy.first().copied().unwrap_or(b'.');
            let y = xy.get(1).copied().unwrap_or(b'.');
            if x != b'.' {
                staged += 1;
            }
            if y != b'.' {
                unstaged += 1;
            }
            // Extract path: it follows the fixed fields after the last space.
            // For type-2 lines the orig path arrives as the next NUL-token; skip it.
            let path = path_from_entry(entry);
            changed_paths.insert(path);
            if entry.starts_with(b"2 ") {
                // Consume the orig-path token.
                tokens.next();
            }
        } else if entry.starts_with(b"u ") {
            // Unmerged: counts as both staged and unstaged slot.
            staged += 1;
            unstaged += 1;
            let path = path_from_entry(entry);
            changed_paths.insert(path);
        }
        // `#` header lines and `!` ignored lines are skipped.
    }

    StatusCounts {
        changes: changed_paths.len() as u64,
        staged,
        unstaged,
        untracked,
    }
}

/// Extract the path bytes from a porcelain-v2 entry line.
///
/// The path is everything after the last ASCII space in the fixed-field
/// prefix. This works for `1`, `2`, and `u` line types.
fn path_from_entry(entry: &[u8]) -> Vec<u8> {
    // Walk backwards to find the last space; path is the suffix after it.
    entry
        .iter()
        .rposition(|&b| b == b' ')
        .map(|i| entry[i + 1..].to_vec())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_porcelain2_z unit tests ─────────────────────────────────────

    fn nul_join(entries: &[&[u8]]) -> Vec<u8> {
        let mut out = Vec::new();
        for e in entries {
            out.extend_from_slice(e);
            out.push(0);
        }
        out
    }

    #[test]
    fn parse_empty_output_all_zeros() {
        let c = parse_porcelain2_z(b"");
        assert_eq!(c.staged, 0);
        assert_eq!(c.unstaged, 0);
        assert_eq!(c.untracked, 0);
        assert_eq!(c.changes, 0);
    }

    #[test]
    fn parse_untracked_only() {
        let out = nul_join(&[b"? new_file.txt", b"? another.rs"]);
        let c = parse_porcelain2_z(&out);
        assert_eq!(c.untracked, 2);
        assert_eq!(c.staged, 0);
        assert_eq!(c.unstaged, 0);
        assert_eq!(c.changes, 2);
    }

    #[test]
    fn parse_staged_modified() {
        // X='M' (staged), Y='.' (no worktree change)
        let out = nul_join(&[b"1 M. N... 100644 100644 100644 abc def file.txt"]);
        let c = parse_porcelain2_z(&out);
        assert_eq!(c.staged, 1);
        assert_eq!(c.unstaged, 0);
        assert_eq!(c.changes, 1);
    }

    #[test]
    fn parse_unstaged_modified() {
        // X='.' (nothing staged), Y='M' (worktree modified)
        let out = nul_join(&[b"1 .M N... 100644 100644 100644 abc def file.txt"]);
        let c = parse_porcelain2_z(&out);
        assert_eq!(c.staged, 0);
        assert_eq!(c.unstaged, 1);
        assert_eq!(c.changes, 1);
    }

    #[test]
    fn parse_both_staged_and_unstaged_one_file() {
        // X='M' (staged), Y='M' (also modified in worktree) — ONE physical file.
        let out = nul_join(&[b"1 MM N... 100644 100644 100644 abc def file.txt"]);
        let c = parse_porcelain2_z(&out);
        assert_eq!(c.staged, 1);
        assert_eq!(c.unstaged, 1);
        // Distinct-file count: still 1 physical file.
        assert_eq!(c.changes, 1);
    }

    #[test]
    fn parse_mixed_entries() {
        let out = nul_join(&[
            b"1 M. N... 100644 100644 100644 a b staged.txt",
            b"1 .M N... 100644 100644 100644 a b modified.txt",
            b"? untracked.txt",
        ]);
        let c = parse_porcelain2_z(&out);
        assert_eq!(c.staged, 1);
        assert_eq!(c.unstaged, 1);
        assert_eq!(c.untracked, 1);
        assert_eq!(c.changes, 3);
    }

    #[test]
    fn parse_header_lines_ignored() {
        let out = nul_join(&[b"# branch.oid abc", b"# branch.head main", b"? foo.txt"]);
        let c = parse_porcelain2_z(&out);
        assert_eq!(c.untracked, 1);
        assert_eq!(c.staged, 0);
        assert_eq!(c.changes, 1);
    }

    #[test]
    fn parse_unmerged_counts_both_slots_one_file() {
        let out = nul_join(&[b"u UU N... 0 0 0 0 abc def base merge.txt"]);
        let c = parse_porcelain2_z(&out);
        assert_eq!(c.staged, 1);
        assert_eq!(c.unstaged, 1);
        // Unmerged file counts once in changes.
        assert_eq!(c.changes, 1);
    }

    #[test]
    fn parse_renamed_file_type2_line() {
        // `2 R.` = renamed, staged; orig path is next NUL token.
        let out = nul_join(&[
            b"2 R. N... 100644 100644 100644 abc def R100 new.txt",
            b"old.txt",
        ]);
        let c = parse_porcelain2_z(&out);
        assert_eq!(c.staged, 1);
        assert_eq!(c.unstaged, 0);
        assert_eq!(c.changes, 1);
    }

    #[test]
    fn parse_submodule_modified_line() {
        let out = nul_join(&[b"1 .M SC.. 160000 160000 160000 abc def mysubmodule"]);
        let c = parse_porcelain2_z(&out);
        assert_eq!(c.staged, 0);
        assert_eq!(c.unstaged, 1);
        assert_eq!(c.changes, 1);
    }

    #[test]
    fn parse_submodule_staged_and_unstaged_one_file() {
        let out = nul_join(&[b"1 MM SC.. 160000 160000 160000 abc def mysubmodule"]);
        let c = parse_porcelain2_z(&out);
        assert_eq!(c.staged, 1);
        assert_eq!(c.unstaged, 1);
        // One submodule = one distinct file.
        assert_eq!(c.changes, 1);
    }

    #[test]
    fn parse_type1_line_with_only_xy_no_trailing_fields() {
        // Minimal / truncated line — should not panic.
        let out = nul_join(&[b"1 M."]);
        let c = parse_porcelain2_z(&out);
        assert_eq!(c.staged, 1);
        assert_eq!(c.unstaged, 0);
    }

    #[test]
    fn parse_type1_line_exactly_two_chars_after_prefix() {
        let out = nul_join(&[b"1 MM"]);
        let c = parse_porcelain2_z(&out);
        assert_eq!(c.staged, 1);
        assert_eq!(c.unstaged, 1);
    }

    #[test]
    fn parse_changes_is_distinct_file_count_not_slot_sum() {
        // file1.txt: staged + unstaged (MM) — 1 file, contributes staged=1, unstaged=1
        // file2.txt: staged only (M.) — 1 file
        // file3.txt: unstaged only (.M) — 1 file
        // new.txt: untracked — 1 file
        // conflict.txt: unmerged (u) — 1 file, contributes staged=1, unstaged=1
        let out = nul_join(&[
            b"1 MM N... 100644 100644 100644 a b file1.txt",
            b"1 M. N... 100644 100644 100644 a b file2.txt",
            b"1 .M N... 100644 100644 100644 a b file3.txt",
            b"? new.txt",
            b"u AA N... 0 0 0 0 a b conflict.txt",
        ]);
        let c = parse_porcelain2_z(&out);
        // Slot counts (cumulative).
        assert_eq!(c.staged, 3); // MM, M., u
        assert_eq!(c.unstaged, 3); // MM, .M, u
        assert_eq!(c.untracked, 1);
        // Distinct files: 5 physical files regardless of slot overlap.
        assert_eq!(c.changes, 5);
    }

    #[test]
    fn parse_filename_with_newline_with_z_flag_works() {
        // With -z, filenames may contain newline bytes (NUL is the separator).
        // Build a synthetic entry where the filename contains '\n'.
        let mut entry: Vec<u8> =
            b"1 M. N... 100644 100644 100644 abc def file\nwith\nnewlines.txt".to_vec();
        entry.push(0); // NUL terminator
        let c = parse_porcelain2_z(&entry);
        assert_eq!(c.staged, 1);
        assert_eq!(c.unstaged, 0);
        assert_eq!(c.changes, 1);
    }
}
