# srp

`srp` is a small Rust command-line tool for recursive search and replace across files using regular expressions.

It is built as a convenient wrapper around `ripgrep` for fast matching and file selection, while keeping replacement logic simple in Rust.

It is built for quick repo-wide edits with a safer preview mode:

- Ripgrep-backed search
- Regex search patterns
- In-place replacement
- Dry-run output with highlighted matches
- Hidden-file support via ripgrep
- Extension and glob filtering via ripgrep

## Installation

### Run from source

You can run the tool directly from the project directory without installing it globally:

```bash
cargo run -- <PATTERN> <REPLACEMENT> [PATH]
```

### Build the binary locally

```bash
cargo build --release
```

The binary will be available at:

```bash
target/release/srp
```

Run it directly with:

```bash
./target/release/srp <PATTERN> <REPLACEMENT> [PATH]
```

### Install `srp` as a shell command

If you want `srp` available on your `PATH` as a normal command, install it with Cargo:

```bash
cargo install --path .
```

After that, you can run:

```bash
srp --help
```

### Runtime dependency

`srp` shells out to `ripgrep`, so `rg` must also be installed and available on your `PATH`.

Check with:

```bash
rg --version
```

## Usage

```bash
srp [OPTIONS] <PATTERN> <REPLACEMENT> [PATH]
```

### Arguments

- `<PATTERN>`: regular expression to search for
- `<REPLACEMENT>`: replacement string
- `[PATH]`: directory to search recursively, defaults to `.` if omitted

### Options

- `-n`, `--dry-run`: show matching lines without modifying files
- `--hidden`: include hidden files and hidden directories
- `-t`, `--type <EXT>`: only process files with the given extension
- `-g`, `--glob <GLOB>`: only process files matching a ripgrep glob
- `-h`, `--help`: print CLI help

## Examples

### Preview a replacement before applying it

```bash
srp --dry-run "foo" "bar" ./src
```

This prints each matching file, the matching line numbers, and the matched text highlighted in red.

### Replace text in all Rust files

```bash
srp -t rs "OldType" "NewType" .
```

### Replace across multiple file types

Clap allows repeated `-t` flags:

```bash
srp -t rs -t toml "old_name" "new_name" .
```

### Include hidden files

```bash
srp --hidden "TODO" "DONE" .
```

Without `--hidden`, entries whose names start with `.` are skipped during traversal.

### Use a regex capture group in the replacement

The replacement is passed to `regex::Regex::replace_all`, so capture references like `$1` work:

```bash
srp --dry-run "fn ([a-z_]+)" "pub fn $1" ./src
```

### Filter with a glob

```bash
srp -g "*.rs" "println!" "eprintln!" .
```

Because filtering is delegated to ripgrep, full ripgrep glob semantics apply. Patterns like `*.rs` and `src/**/*.rs` work as expected.

## Behavior Notes

### What gets processed

- `srp` asks `rg` to find matching files quickly
- `rg` handles recursive traversal, ignore rules, hidden-file behavior, and glob filtering
- Matching files are then read with `fs::read_to_string`
- If a file cannot be decoded as UTF-8, it is skipped
- If reading or writing fails, the tool continues with the remaining files

In practice, that means `srp` is intended for text files, not binary files.

### Dry-run mode

With `--dry-run`, the tool:

- delegates preview output to `rg`
- prints matching file paths and matching lines with ripgrep's formatting
- reports a final summary of total matches and matched files

No files are modified in dry-run mode.

### Replace mode

Without `--dry-run`, the tool:

- replaces all matches in each matching file
- writes the updated content back to disk
- prints one line per modified file with the replacement count
- prints a final summary

## Limitations

These are current implementation details, not just documentation caveats:

- `rg` must be installed and available on `PATH`
- Matching is done on whole-file UTF-8 strings, so binary files are skipped
- There is no interactive confirmation or backup creation
- Replacement is still performed in Rust after ripgrep identifies candidate files

For large or risky replacements, run `--dry-run` first.

## Code Overview

The project is intentionally compact. Most behavior lives in [`src/main.rs`](/Users/lucas.nicolas/tools/srp/src/main.rs).

### CLI definition

The `Cli` struct uses `clap::Parser` to define:

- positional arguments for pattern, replacement, and path
- flags for dry-run and hidden traversal
- optional filters for extension and glob matching

This is the entry point for the command-line interface.

### Ripgrep integration

The core change is that `srp` now shells out to `rg` for search and file discovery.

The helper functions build ripgrep arguments from the CLI flags:

- `--hidden` is passed through to `rg`
- each `-t/--type` becomes a `--glob "*.ext"` filter
- `-g/--glob` is passed through directly
- the search path is forwarded as the final positional argument

This means ignore handling and path matching come from ripgrep rather than custom traversal logic.

### Candidate file discovery

`matched_files(...)` runs ripgrep with `--files-with-matches --null` to retrieve the list of files containing at least one match.

That file list is the bridge between the fast search phase and the Rust replacement phase.

### Regex compilation and error handling

The regex is still compiled once in Rust:

```rust
let re = Regex::new(&cli.pattern)
```

This validates the pattern before replacement and is also used to count replacements in summaries.

The program also checks that `rg` is available before doing any work.

### Match preview

In dry-run mode, `srp` lets ripgrep print the preview output directly:

```rust
rg --color=always --line-number ...
```

This keeps preview behavior fast and aligned with ripgrep's match engine. The Rust side then computes the summary by reading only the matched files and counting matches with `re.find_iter(...)`.

### In-place replacement

In replace mode, each matched file is read and transformed with:

```rust
re.replace_all(&content, cli.replacement.as_str())
```

The resulting text is then written back with `fs::write(...)`.

This keeps the write path simple, but it means each matched file is loaded fully into memory before being rewritten.

## Dependencies

Defined in [`Cargo.toml`](/Users/lucas.nicolas/tools/srp/Cargo.toml):

- `clap`: argument parsing
- `regex`: pattern matching and replacement
- `colored`: colored terminal output

`ripgrep` is an external runtime dependency rather than a Rust crate dependency.

## Typical Workflow

```bash
# 1. Preview
srp --dry-run "old_api" "new_api" ./src

# 2. Apply
srp "old_api" "new_api" ./src
```

That is the intended workflow: inspect first, then write.
