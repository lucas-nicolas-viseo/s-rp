use clap::Parser;
use colored::Colorize;
use regex::Regex;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

#[derive(Parser)]
#[command(
    name = "srp",
    about = "Search and replace across files using ripgrep-backed search"
)]
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

struct CommandOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn main() {
    let cli = Cli::parse();

    let re = match Regex::new(&cli.pattern) {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "{} invalid regex '{}': {}",
                "error:".red().bold(),
                cli.pattern,
                e
            );
            std::process::exit(1);
        }
    };

    ensure_ripgrep_available();

    let matched_files = match matched_files(&cli) {
        Ok(files) => files,
        Err(message) => {
            eprintln!("{} {}", "error:".red().bold(), message);
            std::process::exit(1);
        }
    };

    if cli.dry_run {
        if let Err(message) = run_dry_run(&cli, &re, &matched_files) {
            eprintln!("{} {}", "error:".red().bold(), message);
            std::process::exit(1);
        }
    } else {
        run_replace(&cli, &re, &matched_files);
    }
}

fn ensure_ripgrep_available() {
    match Command::new("rg").arg("--version").output() {
        Ok(output) if output.status.success() => {}
        Ok(_) => {
            eprintln!(
                "{} ripgrep ('rg') is installed but could not be executed successfully",
                "error:".red().bold()
            );
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!(
                "{} ripgrep ('rg') is required but was not found: {}",
                "error:".red().bold(),
                err
            );
            std::process::exit(1);
        }
    }
}

fn rg_base_args(cli: &Cli) -> Vec<String> {
    let mut args = vec!["--with-filename".to_string(), "--line-number".to_string()];

    if cli.hidden {
        args.push("--hidden".to_string());
    }

    if let Some(ref exts) = cli.file_type {
        for ext in exts {
            let normalized = ext.trim_start_matches('.');
            args.push("--glob".to_string());
            args.push(format!("*.{normalized}"));
        }
    }

    if let Some(ref glob) = cli.glob {
        args.push("--glob".to_string());
        args.push(glob.clone());
    }

    args
}

fn run_rg<I, S>(args: I) -> Result<CommandOutput, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("rg")
        .args(args)
        .output()
        .map(|output| CommandOutput {
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
        })
        .map_err(|err| format!("failed to execute rg: {err}"))
}

fn matched_files(cli: &Cli) -> Result<Vec<PathBuf>, String> {
    let mut args = rg_base_args(cli);
    args.push("--files-with-matches".to_string());
    args.push("--null".to_string());
    args.push(cli.pattern.clone());
    args.push(cli.path.display().to_string());

    let output = run_rg(args)?;

    match output.status.code() {
        Some(0) => {
            let files = output
                .stdout
                .split(|byte| *byte == b'\0')
                .filter(|chunk| !chunk.is_empty())
                .map(|chunk| PathBuf::from(String::from_utf8_lossy(chunk).into_owned()))
                .collect();
            Ok(files)
        }
        Some(1) => Ok(Vec::new()),
        _ => Err(rg_error_message(output.stderr)),
    }
}

fn run_dry_run(cli: &Cli, re: &Regex, files: &[PathBuf]) -> Result<(), String> {
    let mut args = rg_base_args(cli);
    args.push("--color=always".to_string());
    args.push(cli.pattern.clone());
    args.push(cli.path.display().to_string());

    let output = run_rg(args)?;

    match output.status.code() {
        Some(0) => {
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }
        Some(1) => {}
        _ => return Err(rg_error_message(output.stderr)),
    }

    let total_matches = count_matches(files, re);

    println!();
    if files.is_empty() {
        println!("{}", "no matches found".dimmed());
    } else {
        println!(
            "{} {} match{} in {} file{}",
            "dry run:".yellow().bold(),
            total_matches,
            if total_matches == 1 { "" } else { "es" },
            files.len(),
            if files.len() == 1 { "" } else { "s" },
        );
        println!("  run without {} to apply", "--dry-run".bold());
    }

    Ok(())
}

fn run_replace(cli: &Cli, re: &Regex, files: &[PathBuf]) {
    let mut total_replacements = 0;
    let mut modified_files = 0;

    for path in files {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                eprintln!(
                    "{} reading {}: {}",
                    "error:".red().bold(),
                    path.display(),
                    err
                );
                continue;
            }
        };

        let count = re.find_iter(&content).count();
        if count == 0 {
            continue;
        }

        let new_content = re.replace_all(&content, cli.replacement.as_str());
        if let Err(err) = fs::write(path, new_content.as_ref()) {
            eprintln!(
                "{} writing {}: {}",
                "error:".red().bold(),
                path.display(),
                err
            );
            continue;
        }

        modified_files += 1;
        total_replacements += count;

        println!(
            "  {} {} ({})",
            "replaced".green(),
            path.display(),
            format!("{count} match{}", if count == 1 { "" } else { "es" }).dimmed()
        );
    }

    println!();
    if modified_files > 0 {
        println!(
            "{} {} replacement{} in {} file{}",
            "done:".green().bold(),
            total_replacements,
            if total_replacements == 1 { "" } else { "s" },
            modified_files,
            if modified_files == 1 { "" } else { "s" },
        );
    } else {
        println!("{}", "no matches found".dimmed());
    }
}

fn count_matches(files: &[PathBuf], re: &Regex) -> usize {
    files
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .map(|content| re.find_iter(&content).count())
        .sum()
}

fn rg_error_message(stderr: Vec<u8>) -> String {
    let stderr = String::from_utf8_lossy(&stderr);
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        "rg failed".to_string()
    } else {
        trimmed.to_string()
    }
}
