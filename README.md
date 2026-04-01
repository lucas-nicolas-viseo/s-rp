# srp

`srp` is a small Rust command-line tool for recursive search and replace across files using regular expressions.

It is built for quick repo-wide edits with a safer preview mode:

- Recursive directory traversal
- Regex search patterns
- In-place replacement
- Dry-run output with highlighted matches
- Optional hidden-file inclusion
- Optional extension and glob-style filtering

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
- `-g`, `--glob <GLOB>`: only process files whose file name matches a simple glob
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

Important: the current implementation matches the glob against the file name only, not the full path. For example, `*.rs` works, but `src/**/*.rs` does not behave like a full path-aware glob engine.

## Behavior Notes

### What gets processed

- `srp` walks the target directory recursively with `walkdir`
- Only regular files are considered
- Files are read with `fs::read_to_string`
- If a file cannot be decoded as UTF-8, it is skipped
- If reading or writing fails, the tool continues with the remaining files

In practice, that means `srp` is intended for text files, not binary files.

### Dry-run mode

With `--dry-run`, the tool:

- prints each matching file path
- prints each matching line with its line number
- highlights the matched substring
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

- `--glob` is a minimal custom matcher, not a full glob implementation
- `--glob` checks only the file name, not the relative path
- Matching is done on whole-file UTF-8 strings, so binary files are skipped
- There is no interactive confirmation or backup creation
- There is no ignore-file support such as `.gitignore`

For large or risky replacements, run `--dry-run` first.

## Code Overview

The project is intentionally compact. Most behavior lives in [`src/main.rs`](/Users/lucas.nicolas/tools/srp/src/main.rs).

### CLI definition

The `Cli` struct uses `clap::Parser` to define:

- positional arguments for pattern, replacement, and path
- flags for dry-run and hidden traversal
- optional filters for extension and glob matching

This is the entry point for the command-line interface.

### Traversal

`WalkDir::new(&cli.path)` recursively visits the target directory.

Hidden-file behavior is controlled by `filter_entry(...)` and `should_skip_hidden(...)`:

- with `--hidden`, everything is traversed
- otherwise, entries beginning with `.` are skipped, except for the root path itself

### Filtering

After traversal, the tool applies two optional filters:

- `matches_extension(...)` checks `path.extension()`
- `matches_glob(...)` checks the file name against a custom `glob_match(...)` function

The glob matcher supports:

- `*` for any sequence of characters
- `?` for a single character

It does not implement path-aware glob semantics.

### Regex compilation and error handling

The regex is compiled once at startup:

```rust
let re = Regex::new(&cli.pattern)
```

If the pattern is invalid, the tool prints a colored error and exits with status code `1`.

### Match preview

In dry-run mode, each line is checked with `re.is_match(line)`. Matching substrings are highlighted by calling `re.replace_all(...)` with a closure that wraps each match in colored terminal output.

The tool also counts matches with `re.find_iter(...)`.

### In-place replacement

In replace mode, the full file content is transformed with:

```rust
re.replace_all(&content, cli.replacement.as_str())
```

The resulting text is then written back with `fs::write(...)`.

This keeps the implementation simple, but it means the full file is loaded into memory.

## Dependencies

Defined in [`Cargo.toml`](/Users/lucas.nicolas/tools/srp/Cargo.toml):

- `clap`: argument parsing
- `regex`: pattern matching and replacement
- `walkdir`: recursive traversal
- `colored`: colored terminal output

## Typical Workflow

```bash
# 1. Preview
srp --dry-run "old_api" "new_api" ./src

# 2. Apply
srp "old_api" "new_api" ./src
```

That is the intended workflow: inspect first, then write.
