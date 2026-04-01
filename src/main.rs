use clap::Parser;
use colored::Colorize;
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "srp", about = "Search and replace across files using regex")]
struct Cli {
    /// Regex pattern to search for
    pattern: String,

    /// Replacement string
    replacement: String,

    /// Path to search in (default: current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Dry run — show matches without replacing
    #[arg(short = 'n', long = "dry-run")]
    dry_run: bool,

    /// Include hidden files and directories
    #[arg(long = "hidden")]
    hidden: bool,

    /// Filter by file extension (e.g. "rs", "py", "ts")
    #[arg(short = 't', long = "type", value_name = "EXT")]
    file_type: Option<Vec<String>>,

    /// Glob pattern to include (e.g. "*.rs", "src/**/*.py")
    #[arg(short = 'g', long = "glob")]
    glob: Option<String>,
}

fn should_skip_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn matches_extension(path: &std::path::Path, extensions: &[String]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| extensions.iter().any(|e| e == ext))
        .unwrap_or(false)
}

fn matches_glob(path: &std::path::Path, pattern: &str) -> bool {
    // Simple glob: just check if filename matches
    let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
    glob_match(pattern, file_name)
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let mut p = pattern.chars().peekable();
    let mut t = text.chars().peekable();

    while p.peek().is_some() {
        match p.peek() {
            Some('*') => {
                p.next();
                if p.peek().is_none() {
                    return true;
                }
                while t.peek().is_some() {
                    let remaining_pattern: String = p.clone().collect();
                    let remaining_text: String = t.clone().collect();
                    if glob_match(&remaining_pattern, &remaining_text) {
                        return true;
                    }
                    t.next();
                }
                return false;
            }
            Some('?') => {
                p.next();
                if t.next().is_none() {
                    return false;
                }
            }
            Some(&pc) => {
                p.next();
                match t.next() {
                    Some(tc) if tc == pc => {}
                    _ => return false,
                }
            }
            None => break,
        }
    }

    t.peek().is_none()
}

fn main() {
    let cli = Cli::parse();

    let re = match Regex::new(&cli.pattern) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} invalid regex '{}': {}", "error:".red().bold(), cli.pattern, e);
            std::process::exit(1);
        }
    };

    let walker = WalkDir::new(&cli.path).into_iter().filter_entry(|e| {
        if cli.hidden {
            true
        } else {
            !should_skip_hidden(e) || e.depth() == 0
        }
    });

    let mut matched_files = 0;
    let mut total_replacements = 0;

    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // Filter by extension
        if let Some(ref exts) = cli.file_type {
            if !matches_extension(path, exts) {
                continue;
            }
        }

        // Filter by glob
        if let Some(ref glob_pat) = cli.glob {
            if !matches_glob(path, glob_pat) {
                continue;
            }
        }

        // Read file, skip binary
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if !re.is_match(&content) {
            continue;
        }

        matched_files += 1;

        if cli.dry_run {
            println!("  {}", path.display().to_string().cyan());
            for (i, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    let line_num = format!("{:>4}", i + 1);
                    let highlighted = re.replace_all(line, |caps: &regex::Captures| {
                        format!("{}", caps[0].red().bold())
                    });
                    println!("    {} {}", line_num.dimmed(), highlighted);
                    total_replacements += re.find_iter(line).count();
                }
            }
        } else {
            let new_content = re.replace_all(&content, cli.replacement.as_str());
            let count: usize = re.find_iter(&content).count();
            total_replacements += count;

            if let Err(e) = fs::write(path, new_content.as_ref()) {
                eprintln!("{} writing {}: {}", "error:".red().bold(), path.display(), e);
                continue;
            }
            println!(
                "  {} {} ({})",
                "replaced".green(),
                path.display(),
                format!("{count} match{}", if count == 1 { "" } else { "es" }).dimmed()
            );
        }
    }

    // Summary
    println!();
    if cli.dry_run {
        println!(
            "{} {} match{} in {} file{}",
            "dry run:".yellow().bold(),
            total_replacements,
            if total_replacements == 1 { "" } else { "es" },
            matched_files,
            if matched_files == 1 { "" } else { "s" },
        );
        println!(
            "  run without {} to apply",
            "--dry-run".bold()
        );
    } else if matched_files > 0 {
        println!(
            "{} {} replacement{} in {} file{}",
            "done:".green().bold(),
            total_replacements,
            if total_replacements == 1 { "" } else { "s" },
            matched_files,
            if matched_files == 1 { "" } else { "s" },
        );
    } else {
        println!("{}", "no matches found".dimmed());
    }
}
