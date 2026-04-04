# M3 fs crate Design — glob / grep / gitignore / diff

> Status: Design pre-study
> Scope: crates/fs/src/{glob,grep,gitignore,diff}.rs

---

## 1. Dependencies and API Surface

### 1.1 globset 0.4

- `GlobBuilder::new(pattern).build()` -> `Result<Glob, Error>`
- `GlobSetBuilder::new().add(glob).build()` -> `Result<GlobSet, Error>`
- `GlobSet::is_match(path)` -> `bool`
- Patterns follow gitglob syntax (`**/*.rs`, `src/{a,b}/*.rs`)
- Case sensitivity configurable via `GlobBuilder::case_insensitive(bool)`

### 1.2 ignore 0.4

- `WalkBuilder::new(root)` — main entry point
  - `.hidden(bool)` — skip hidden files (default: true)
  - `.git_ignore(bool)` — respect .gitignore (default: true)
  - `.git_global(bool)` — respect global gitignore
  - `.git_exclude(bool)` — respect .git/info/exclude
  - `.parents(bool)` — respect parent .gitignore files
  - `.add_custom_ignore_file(path)` — add .crabignore etc.
  - `.max_depth(Some(n))` — limit recursion
  - `.sort_by_file_path(fn)` — custom sort
  - `.build()` -> `Walk` iterator yielding `Result<DirEntry, Error>`
- `DirEntry::path()`, `.file_type()`, `.metadata()`
- `ignore::gitignore::Gitignore::new(path)` -> `(Gitignore, Option<Error>)`
  - `.matched(path, is_dir)` -> `Match<()>` (Ignore/Whitelist/None)

### 1.3 similar 2

- `TextDiff::from_lines(old, new)` — line-level diff
- `.unified_diff().header(old_label, new_label).to_string()` — unified format
- `TextDiff::from_words()`, `from_chars()` — finer granularity
- `ChangeTag::{Equal, Delete, Insert}` — per-change classification
- No built-in patch-apply; we apply edits via string replacement

### 1.4 grep-regex + grep-searcher (potential addition)

Architecture doc mentions "ripgrep internals" but Cargo.toml currently only lists `ignore`.
Two approaches:

- **Option A (recommended for M3)**: Use `ignore::WalkBuilder` for traversal + `regex::Regex` for matching after reading file content. Simpler dependency tree, sufficient for initial implementation.
- **Option B (future)**: Add `grep-regex` + `grep-searcher` crates for true ripgrep-level performance with memory-mapped search. Better for large repos.

Recommendation: Start with Option A, add grep-* crates in a performance pass.

---

## 2. Module Designs

### 2.1 glob.rs — File Pattern Matching

```rust
//! Glob-based file pattern matching (globset + ignore).

use std::path::{Path, PathBuf};

/// Result of a glob search.
pub struct GlobResult {
    /// Matched file paths, sorted by modification time (most recent first).
    pub matches: Vec<PathBuf>,
    /// True if results were truncated at `limit`.
    pub truncated: bool,
}

/// Options for glob search.
pub struct GlobOptions<'a> {
    /// Root directory to search from.
    pub root: &'a Path,
    /// Glob pattern (gitglob syntax, e.g. `**/*.rs`).
    pub pattern: &'a str,
    /// Maximum number of results to return. 0 = unlimited.
    pub limit: usize,
    /// Whether to respect .gitignore rules. Default: true.
    pub respect_gitignore: bool,
    /// Whether to include hidden files. Default: false.
    pub include_hidden: bool,
    /// Optional max directory depth.
    pub max_depth: Option<usize>,
}

impl<'a> GlobOptions<'a> {
    pub fn new(root: &'a Path, pattern: &'a str) -> Self { /* defaults */ }
}

/// Find files matching a glob pattern.
///
/// Implementation plan:
/// 1. Parse pattern with `globset::GlobBuilder`
/// 2. Walk with `ignore::WalkBuilder` (configurable .gitignore respect)
/// 3. Filter entries against compiled GlobSet
/// 4. Collect metadata for mtime sort
/// 5. Sort by mtime descending
/// 6. Truncate at limit, set `truncated` flag
pub fn find_files(opts: &GlobOptions<'_>) -> crab_common::Result<GlobResult>;
```

**Key decisions:**
- Use `ignore::WalkBuilder` (not `std::fs::read_dir`) for .gitignore integration
- Sort by mtime — matches Claude Code Glob tool behavior
- `GlobOptions` struct instead of positional args — extensible without breaking changes
- Return `truncated` flag so callers (Tool layer) can report it to the LLM

**Error mapping:**
- `globset::Error` -> `crab_common::Error::Other(format!("invalid glob: {e}"))`
- `ignore::Error` -> `crab_common::Error::Io(...)` or `Error::Other(...)`

### 2.2 grep.rs — Content Search

```rust
//! Content search using regex + ignore walker.

use std::path::PathBuf;

/// A single line match from a grep search.
#[derive(Debug, Clone)]
pub struct GrepMatch {
    /// File containing the match.
    pub path: PathBuf,
    /// 1-based line number.
    pub line_number: usize,
    /// The matched line content (trimmed trailing newline).
    pub line_content: String,
    /// Context lines before the match (if requested).
    pub context_before: Vec<String>,
    /// Context lines after the match (if requested).
    pub context_after: Vec<String>,
}

/// Options for a content search.
pub struct GrepOptions {
    /// Regex pattern to search for.
    pub pattern: String,
    /// Root path to search (file or directory).
    pub path: PathBuf,
    /// Case insensitive matching.
    pub case_insensitive: bool,
    /// Optional file glob filter (e.g. "*.rs").
    pub file_glob: Option<String>,
    /// Maximum number of matches to return. 0 = unlimited.
    pub max_results: usize,
    /// Number of context lines before/after each match.
    pub context_lines: usize,
    /// Whether to respect .gitignore. Default: true.
    pub respect_gitignore: bool,
}

/// Search file contents by regex pattern.
///
/// Implementation plan:
/// 1. Compile regex with `regex::RegexBuilder` (case_insensitive flag)
/// 2. If path is a file, search that single file
/// 3. If path is a directory, walk with `ignore::WalkBuilder`
///    - Apply file_glob filter if provided
/// 4. For each file: read to string, search line by line
/// 5. Collect matches with context lines
/// 6. Stop at max_results
pub fn search(opts: &GrepOptions) -> crab_common::Result<Vec<GrepMatch>>;

/// Search a single file's contents. Used internally and for testing.
fn search_file(
    path: &Path,
    regex: &regex::Regex,
    context_lines: usize,
) -> crab_common::Result<Vec<GrepMatch>>;
```

**Key decisions:**
- Context lines stored per-match (matches Claude Code Grep tool `-B`/`-A`/`-C` flags)
- Single-file path support (for searching one known file)
- M3 uses `regex` crate directly; future perf pass can swap to `grep-searcher` for mmap
- Binary file detection: skip files that fail `String::from_utf8` or contain null bytes

**Dependencies to add to Cargo.toml:**
- `regex` (workspace dep) — needed for pattern compilation

### 2.3 gitignore.rs — Gitignore Filter

```rust
//! .gitignore rule parsing and path filtering.

use std::path::{Path, PathBuf};

/// Parsed gitignore rules for a directory tree.
///
/// Wraps `ignore::gitignore::Gitignore` with support for:
/// - Nested .gitignore files (walked from root upward)
/// - Global gitignore (~/.config/git/ignore)
/// - .git/info/exclude
/// - Custom ignore files (.crabignore)
pub struct GitIgnoreFilter {
    root: PathBuf,
    /// The compiled gitignore matcher from the `ignore` crate.
    gitignore: ignore::gitignore::Gitignore,
}

impl GitIgnoreFilter {
    /// Load all gitignore rules for the directory tree rooted at `root`.
    ///
    /// Walks up from root to find parent .gitignore files.
    /// Also loads global gitignore and .git/info/exclude.
    ///
    /// Implementation:
    /// 1. Use `ignore::gitignore::GitignoreBuilder::new(root)`
    /// 2. Add all .gitignore files from root upward
    /// 3. Add global gitignore path
    /// 4. `.build()` -> compiled matcher
    pub fn new(root: &Path) -> crab_common::Result<Self>;

    /// Create a filter with an additional custom ignore file (e.g. .crabignore).
    pub fn with_custom_ignore(root: &Path, ignore_file: &Path) -> crab_common::Result<Self>;

    /// Check whether a path should be ignored.
    ///
    /// `is_dir` hint improves accuracy (gitignore trailing `/` rules).
    #[must_use]
    pub fn is_ignored(&self, path: &Path) -> bool;

    /// Check with explicit directory hint.
    #[must_use]
    pub fn is_ignored_dir(&self, path: &Path, is_dir: bool) -> bool;

    /// The root directory this filter was built for.
    #[must_use]
    pub fn root(&self) -> &Path;
}
```

**Key decisions:**
- Thin wrapper over `ignore::gitignore::Gitignore` — no reinventing
- `is_ignored()` auto-detects is_dir via `path.is_dir()` (convenience)
- `is_ignored_dir()` for callers that already know (avoids stat syscall)
- `.crabignore` support via `with_custom_ignore` — future-proofing for Crab Code specific ignores
- Used internally by glob.rs and grep.rs (they use `WalkBuilder` which handles this automatically, but `GitIgnoreFilter` is exposed for other callers like the Tool layer)

### 2.4 diff.rs — Diff Generation and Edit Application

```rust
//! Diff generation and edit application using `similar`.

/// Result of applying an edit operation.
#[derive(Debug, Clone)]
pub struct EditResult {
    /// Content before the edit.
    pub old_content: String,
    /// Content after the edit.
    pub new_content: String,
    /// Unified diff string.
    pub unified_diff: String,
    /// Number of replacements made.
    pub replacements: usize,
}

/// Options for edit application.
pub struct EditOptions<'a> {
    /// The full file content to edit.
    pub file_content: &'a str,
    /// The exact string to find and replace.
    pub old_string: &'a str,
    /// The replacement string.
    pub new_string: &'a str,
    /// If true, replace all occurrences. If false, error on multiple matches.
    pub replace_all: bool,
    /// File path label for the diff header (display only).
    pub file_label: Option<&'a str>,
}

/// Apply an exact string replacement.
///
/// Implementation plan:
/// 1. Count occurrences of old_string in file_content
/// 2. If 0: return error "old_string not found"
/// 3. If >1 and !replace_all: return error "ambiguous match (N occurrences)"
/// 4. Perform replacement
/// 5. Generate unified diff via `similar::TextDiff::from_lines`
/// 6. Return EditResult with old, new, diff, and count
pub fn apply_edit(opts: &EditOptions<'_>) -> crab_common::Result<EditResult>;

/// Convenience wrapper matching the current skeleton signature.
pub fn apply_edit_simple(
    file_content: &str,
    old_string: &str,
    new_string: &str,
) -> crab_common::Result<EditResult>;

/// Generate a unified diff between two strings without applying edits.
///
/// Useful for preview / dry-run.
pub fn unified_diff(
    old: &str,
    new: &str,
    old_label: &str,
    new_label: &str,
) -> String;
```

**Key decisions:**
- `replace_all` flag mirrors Claude Code Edit tool's `replace_all` parameter
- Ambiguity check: if `old_string` appears multiple times and `replace_all=false`, error out (matches CC behavior — "old_string is not unique")
- `unified_diff()` helper for generating diffs without edits (useful for `WriteFile` tool preview)
- `apply_edit_simple()` preserves backward compat with existing skeleton

---

## 3. Error Handling Strategy

All modules use `crab_common::Result<T>` which wraps `crab_common::Error`.

| Situation | Error variant |
|-----------|---------------|
| Invalid glob pattern | `Error::Other("invalid glob pattern: ...")` |
| Invalid regex | `Error::Other("invalid regex: ...")` |
| Path not found / inaccessible | `Error::Io(std::io::Error)` |
| old_string not found in edit | `Error::Other("old_string not found in file content")` |
| Ambiguous edit match | `Error::Other("old_string matches N times; use replace_all or provide more context")` |
| Binary file encountered | Silently skip (grep), not an error |

Consider adding an `Error::Fs(String)` variant to `crab_common::Error` for fs-specific errors. Not blocking for M3 — `Error::Other` suffices initially.

---

## 4. Dependency Changes Required

Current `crates/fs/Cargo.toml` dependencies:
```toml
crab-common.workspace = true
globset.workspace = true
ignore.workspace = true
notify.workspace = true
similar.workspace = true
fd-lock.workspace = true
```

**To add for M3:**
```toml
regex.workspace = true    # grep.rs pattern compilation
```

**Root Cargo.toml workspace.dependencies addition:**
```toml
regex = "1"
```

---

## 5. Testing Strategy

Each module gets a `#[cfg(test)] mod tests` block plus integration tests in `crates/fs/tests/`.

### 5.1 glob.rs tests
- `test_simple_pattern` — `*.rs` in a temp dir
- `test_nested_glob` — `**/*.toml` finds nested files
- `test_gitignore_respected` — files in .gitignore are excluded
- `test_limit_truncation` — limit=2 with 5 matches sets truncated=true
- `test_mtime_sort` — most recently modified file first
- `test_invalid_pattern` — returns error

### 5.2 grep.rs tests
- `test_simple_match` — find literal string
- `test_regex_match` — `fn\s+\w+` finds function defs
- `test_case_insensitive` — flag works
- `test_context_lines` — before/after context captured
- `test_file_glob_filter` — only searches matching files
- `test_max_results` — stops at limit
- `test_binary_skip` — binary files silently skipped
- `test_single_file` — path points to a file, not dir

### 5.3 gitignore.rs tests
- `test_basic_ignore` — `*.log` in .gitignore
- `test_negation` — `!important.log` not ignored
- `test_directory_rule` — `build/` ignores directory
- `test_nested_gitignore` — subdirectory .gitignore overrides
- `test_no_gitignore` — gracefully handles missing file

### 5.4 diff.rs tests
- `test_simple_replacement` — single occurrence
- `test_not_found` — error when old_string absent
- `test_ambiguous` — error when multiple matches + replace_all=false
- `test_replace_all` — replaces all occurrences
- `test_unified_diff_format` — output is valid unified diff
- `test_empty_replacement` — deletion (new_string = "")
- `test_multiline_edit` — old_string spans multiple lines

---

## 6. Integration with Tool Layer

The Tool layer (`crates/tools/`) wraps these fs functions:

| Tool | fs function | Notes |
|------|------------|-------|
| `GlobTool` | `glob::find_files` | Formats PathBuf list for LLM |
| `GrepTool` | `grep::search` | Formats matches with line numbers |
| `EditTool` | `diff::apply_edit` | Reads file, applies edit, writes back, returns diff |
| `ReadTool` | direct `std::fs::read_to_string` | May use `gitignore::GitIgnoreFilter` for validation |
| `WriteTool` | `diff::unified_diff` | Preview diff before write |

The fs crate is a pure library — no file writes. The Tool layer handles actual I/O (read file, call fs function, write result).

Exception: `diff::apply_edit` operates on string content, not files. The Tool layer reads the file, passes content to `apply_edit`, then writes the result.
