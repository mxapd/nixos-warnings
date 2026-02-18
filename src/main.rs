use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Result, Write};
use std::path::{Path, PathBuf};

const DEFAULT_INPUT: &str = "/tmp/nixos-rebuild.log";
const DEFAULT_OUTPUT: &str = "/tmp/nixos-rebuild-warnings.txt";

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let command = args.get(1).map(|s| s.as_str());

    match command {
        Some("quickfix") => {
            let input = args.get(2).map(|s| s.as_str()).unwrap_or(DEFAULT_INPUT);
            let output = args.get(3).map(|s| s.as_str()).unwrap_or(DEFAULT_OUTPUT);

            generate_quickfix(input, output)?;
            eprintln!("Warnings written to {}", output);
        }
        Some("count") => {
            let input = args.get(2).map(|s| s.as_str()).unwrap_or(DEFAULT_INPUT);

            let count = count_warnings(input)?;
            println!("{}", count);
        }
        Some("help") | Some("--help") | Some("-h") => {
            print_help();
        }
        None => {
            eprintln!("Error: No command specified\n");
            print_help();
            std::process::exit(1);
        }
        Some(cmd) => {
            eprintln!("Error: Unknown command '{}'\n", cmd);
            print_help();
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_help() {
    println!(
        "nixos-warnings - Parse NixOS rebuild warnings

USAGE:
    nixos-warnings <COMMAND> [OPTIONS]

COMMANDS:
    quickfix [INPUT] [OUTPUT]    Generate quickfix file for Neovim
                                 Default: /tmp/nixos-rebuild.log -> /tmp/nixos-rebuild-warnings.txt
    
    count [INPUT]                Output number of warnings (for Waybar)
                                 Default: /tmp/nixos-rebuild.log
    
    help                         Show this help message

ENVIRONMENT VARIABLES:
    NIXOS_CONFIG_DIR            Base directory for NixOS config (default: /home/$USER/nixos)

EXAMPLES:
    # Generate quickfix list
    nixos-warnings quickfix
    
    # Count warnings for waybar or whatever
    nixos-warnings count
    
    # Custom paths
    nixos-warnings quickfix /var/log/nixos.log /tmp/warnings.txt"
    );
}

fn count_warnings(input: &str) -> Result<usize> {
    let lines = load_file(input)?;
    let warnings = extract_evaluation_warnings(lines);
    Ok(warnings.len())
}

fn generate_quickfix(input: &str, output: &str) -> Result<()> {
    let lines = load_file(input)?;
    let warning_lines = extract_evaluation_warnings(lines);
    if warning_lines.is_empty() {
        eprintln!("No warnings found");
        return Ok(());
    }
    write_quickfix_file(output, &warning_lines)?;
    Ok(())
}

fn get_nixos_config_dir() -> PathBuf {
    env::var("NIXOS_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").unwrap_or_else(|_| "/home".to_string());
            PathBuf::from(home).join("nixos")
        })
}

fn load_file(path: &str) -> Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    reader.lines().collect()
}

fn extract_evaluation_warnings(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| line.starts_with("evaluation warning"))
        .collect()
}

fn write_quickfix_file(path: &str, warning_lines: &[String]) -> Result<()> {
    let mut output = File::create(path)?;

    for line in warning_lines {
        if let Some(warning) = Warning::from_line(line) {
            writeln!(
                output,
                "{}:{}:1: warning: {} -> {}",
                warning.file,
                warning.line.unwrap_or(1),
                warning.old_option,
                warning.new_option
            )?;
        }
    }

    Ok(())
}

fn find_line_number(file_path: &Path, search_text: &str) -> Option<usize> {
    let file = File::open(file_path).ok()?;
    let reader = BufReader::new(file);

    reader.lines().enumerate().find_map(|(line_nr, line)| {
        line.ok()
            .filter(|l| l.contains(search_text))
            .map(|_| line_nr + 1)
    })
}

#[derive(Debug)]
struct Warning {
    file: String,
    line: Option<usize>,
    old_option: String,
    new_option: String,
}

impl Warning {
    fn from_line(line: &str) -> Option<Self> {
        let message = line.strip_prefix("evaluation warning: ")?;

        let old_option = message.split('`').nth(1)?.split('\'').next()?.to_string();

        let new_option = message
            .split('`')
            .nth(3)?
            .trim_end_matches('\'')
            .trim_end_matches('.')
            .to_string();

        let path = message.split('\'').find(|s| s.contains("/nix/store/"))?;

        let file = path
            .split("/nix/store/")
            .nth(1)?
            .split('/')
            .skip(1)
            .collect::<Vec<_>>()
            .join("/");

        let config_dir = get_nixos_config_dir();
        let full_file_path = config_dir.join(&file);
        let search_term = old_option.split('.').last()?;
        let line_nr = find_line_number(&full_file_path, search_term);

        Some(Warning {
            file: full_file_path.to_string_lossy().to_string(),
            line: line_nr,
            old_option,
            new_option,
        })
    }
}
