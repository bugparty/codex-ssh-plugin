mod parser;
mod seek_sequence;

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
pub use parser::{parse_patch, Hunk, ParseError, UpdateFileChunk};
use thiserror::Error;

#[derive(Debug, PartialEq)]
pub struct ApplyPatchArgs {
    pub patch: String,
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Error)]
pub enum ApplyPatchError {
    #[error(transparent)]
    ParseError(#[from] ParseError),
    #[error(transparent)]
    IoError(#[from] IoError),
    #[error("{0}")]
    ComputeReplacements(String),
}

#[derive(Debug, Error)]
#[error("{context}: {source}")]
pub struct IoError {
    context: String,
    #[source]
    source: std::io::Error,
}

impl From<std::io::Error> for ApplyPatchError {
    fn from(err: std::io::Error) -> Self {
        ApplyPatchError::IoError(IoError {
            context: "I/O error".to_string(),
            source: err,
        })
    }
}

pub struct AffectedPaths {
    pub added: Vec<PathBuf>,
    pub modified: Vec<PathBuf>,
    pub deleted: Vec<PathBuf>,
}

pub fn run_cli() -> i32 {
    let mut args = std::env::args_os();
    let _argv0 = args.next();

    let patch_arg = match args.next() {
        Some(arg) => match arg.into_string() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Error: apply_patch requires a UTF-8 patch argument.");
                return 1;
            }
        },
        None => {
            let mut buf = String::new();
            match std::io::stdin().read_to_string(&mut buf) {
                Ok(_) if !buf.is_empty() => buf,
                Ok(_) => {
                    eprintln!("Usage: apply_patch 'PATCH'\n       cat patch.txt | apply_patch");
                    return 2;
                }
                Err(err) => {
                    eprintln!("Error: failed to read patch from stdin.\n{err}");
                    return 1;
                }
            }
        }
    };

    if args.next().is_some() {
        eprintln!("Error: apply_patch accepts exactly one argument.");
        return 2;
    }

    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(err) => {
            eprintln!("Error: failed to determine current directory.\n{err}");
            return 1;
        }
    };

    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    match apply_patch(&patch_arg, &cwd, &mut stdout, &mut stderr) {
        Ok(()) => {
            let _ = stdout.flush();
            0
        }
        Err(_) => 1,
    }
}

pub fn apply_patch(
    patch: &str,
    cwd: &Path,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<(), ApplyPatchError> {
    let hunks = match parse_patch(patch) {
        Ok(source) => source.hunks,
        Err(e) => {
            match &e {
                ParseError::InvalidPatchError(message) => {
                    writeln!(stderr, "Invalid patch: {message}").map_err(ApplyPatchError::from)?;
                }
                ParseError::InvalidHunkError {
                    message,
                    line_number,
                } => {
                    writeln!(stderr, "Invalid patch hunk on line {line_number}: {message}")
                        .map_err(ApplyPatchError::from)?;
                }
            }
            return Err(ApplyPatchError::ParseError(e));
        }
    };

    match apply_hunks(&hunks, cwd) {
        Ok(affected) => {
            print_summary(&affected, stdout).map_err(ApplyPatchError::from)?;
            Ok(())
        }
        Err(err) => {
            writeln!(stderr, "{err}").map_err(ApplyPatchError::from)?;
            Err(ApplyPatchError::IoError(IoError {
                context: err.to_string(),
                source: std::io::Error::other(err),
            }))
        }
    }
}

fn apply_hunks(hunks: &[Hunk], cwd: &Path) -> Result<AffectedPaths> {
    if hunks.is_empty() {
        anyhow::bail!("No files were modified.");
    }

    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();

    for hunk in hunks {
        let affected_path = hunk.path().to_path_buf();
        let path_abs = hunk.resolve_path(cwd);
        match hunk {
            Hunk::AddFile { contents, .. } => {
                write_file_with_missing_parent_retry(&path_abs, contents.as_bytes())?;
                added.push(affected_path);
            }
            Hunk::DeleteFile { .. } => {
                let metadata = fs::metadata(&path_abs)
                    .with_context(|| format!("Failed to stat {}", path_abs.display()))?;
                if metadata.is_dir() {
                    anyhow::bail!("Failed to delete file {}: path is a directory", path_abs.display());
                }
                fs::remove_file(&path_abs)
                    .with_context(|| format!("Failed to delete file {}", path_abs.display()))?;
                deleted.push(affected_path);
            }
            Hunk::UpdateFile {
                move_path, chunks, ..
            } => {
                let new_contents = derive_new_contents_from_chunks(&path_abs, chunks)?;
                if let Some(dest) = move_path {
                    let dest_abs = if dest.is_absolute() {
                        dest.clone()
                    } else {
                        cwd.join(dest)
                    };
                    write_file_with_missing_parent_retry(&dest_abs, new_contents.as_bytes())?;
                    let metadata = fs::metadata(&path_abs)
                        .with_context(|| format!("Failed to stat {}", path_abs.display()))?;
                    if metadata.is_dir() {
                        anyhow::bail!("Failed to remove original {}: path is a directory", path_abs.display());
                    }
                    fs::remove_file(&path_abs).with_context(|| {
                        format!("Failed to remove original {}", path_abs.display())
                    })?;
                } else {
                    fs::write(&path_abs, new_contents.as_bytes())
                        .with_context(|| format!("Failed to write file {}", path_abs.display()))?;
                }
                modified.push(affected_path);
            }
        }
    }

    Ok(AffectedPaths {
        added,
        modified,
        deleted,
    })
}

fn write_file_with_missing_parent_retry(path: &Path, contents: &[u8]) -> Result<()> {
    match fs::write(path, contents) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create parent directories for {}", path.display())
                })?;
            }
            fs::write(path, contents).with_context(|| format!("Failed to write file {}", path.display()))
        }
        Err(err) => Err(err).with_context(|| format!("Failed to write file {}", path.display())),
    }
}

fn derive_new_contents_from_chunks(
    path_abs: &Path,
    chunks: &[UpdateFileChunk],
) -> std::result::Result<String, ApplyPatchError> {
    let original_contents = fs::read_to_string(path_abs).map_err(|err| ApplyPatchError::IoError(
        IoError {
            context: format!("Failed to read file to update {}", path_abs.display()),
            source: err,
        },
    ))?;

    let mut original_lines: Vec<String> = original_contents.split('\n').map(String::from).collect();
    if original_lines.last().is_some_and(String::is_empty) {
        original_lines.pop();
    }

    let replacements = compute_replacements(&original_lines, path_abs, chunks)?;
    let mut new_lines = apply_replacements(original_lines, &replacements);
    if !new_lines.last().is_some_and(String::is_empty) {
        new_lines.push(String::new());
    }
    Ok(new_lines.join("\n"))
}

fn compute_replacements(
    original_lines: &[String],
    path: &Path,
    chunks: &[UpdateFileChunk],
) -> std::result::Result<Vec<(usize, usize, Vec<String>)>, ApplyPatchError> {
    let mut replacements = Vec::new();
    let mut line_index = 0usize;

    for chunk in chunks {
        if let Some(ctx_line) = &chunk.change_context {
            if let Some(idx) = seek_sequence::seek_sequence(
                original_lines,
                std::slice::from_ref(ctx_line),
                line_index,
                false,
            ) {
                line_index = idx + 1;
            } else {
                return Err(ApplyPatchError::ComputeReplacements(format!(
                    "Failed to find context '{}' in {}",
                    ctx_line,
                    path.display()
                )));
            }
        }

        if chunk.old_lines.is_empty() {
            let insertion_idx = if original_lines.last().is_some_and(String::is_empty) {
                original_lines.len() - 1
            } else {
                original_lines.len()
            };
            replacements.push((insertion_idx, 0, chunk.new_lines.clone()));
            continue;
        }

        let mut pattern: &[String] = &chunk.old_lines;
        let mut found =
            seek_sequence::seek_sequence(original_lines, pattern, line_index, chunk.is_end_of_file);
        let mut new_slice: &[String] = &chunk.new_lines;

        if found.is_none() && pattern.last().is_some_and(String::is_empty) {
            pattern = &pattern[..pattern.len() - 1];
            if new_slice.last().is_some_and(String::is_empty) {
                new_slice = &new_slice[..new_slice.len() - 1];
            }
            found = seek_sequence::seek_sequence(
                original_lines,
                pattern,
                line_index,
                chunk.is_end_of_file,
            );
        }

        if let Some(start_idx) = found {
            replacements.push((start_idx, pattern.len(), new_slice.to_vec()));
            line_index = start_idx + pattern.len();
        } else {
            return Err(ApplyPatchError::ComputeReplacements(format!(
                "Failed to find expected lines in {}:\n{}",
                path.display(),
                chunk.old_lines.join("\n"),
            )));
        }
    }

    replacements.sort_by(|(lhs_idx, _, _), (rhs_idx, _, _)| lhs_idx.cmp(rhs_idx));
    Ok(replacements)
}

fn apply_replacements(
    mut lines: Vec<String>,
    replacements: &[(usize, usize, Vec<String>)],
) -> Vec<String> {
    for (start_idx, old_len, new_segment) in replacements.iter().rev() {
        let start_idx = *start_idx;
        let old_len = *old_len;

        for _ in 0..old_len {
            if start_idx < lines.len() {
                lines.remove(start_idx);
            }
        }
        for (offset, new_line) in new_segment.iter().enumerate() {
            lines.insert(start_idx + offset, new_line.clone());
        }
    }
    lines
}

pub fn print_summary(affected: &AffectedPaths, out: &mut impl Write) -> std::io::Result<()> {
    writeln!(out, "Success. Updated the following files:")?;
    for path in &affected.added {
        writeln!(out, "A {}", path.display())?;
    }
    for path in &affected.modified {
        writeln!(out, "M {}", path.display())?;
    }
    for path in &affected.deleted {
        writeln!(out, "D {}", path.display())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::apply_patch;

    fn temp_dir() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rust-apply-patch-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn add_file_hunk_creates_file() {
        let dir = temp_dir();
        let patch = "*** Begin Patch\n*** Add File: hello.txt\n+hello\n*** End Patch";
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        apply_patch(patch, &dir, &mut stdout, &mut stderr).unwrap();
        assert_eq!(fs::read_to_string(dir.join("hello.txt")).unwrap(), "hello\n");
        assert_eq!(String::from_utf8(stderr).unwrap(), "");
    }

    #[test]
    fn update_file_hunk_modifies_file() {
        let dir = temp_dir();
        fs::write(dir.join("note.txt"), "old\n").unwrap();
        let patch = "*** Begin Patch\n*** Update File: note.txt\n@@\n-old\n+new\n*** End Patch";
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        apply_patch(patch, &dir, &mut stdout, &mut stderr).unwrap();
        assert_eq!(fs::read_to_string(dir.join("note.txt")).unwrap(), "new\n");
        assert_eq!(String::from_utf8(stderr).unwrap(), "");
    }
}
