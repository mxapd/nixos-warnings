use std::fs::File;
use std::io::Result;
use std::io::{BufRead, BufReader, Write};

fn main() -> Result<()> {
    let lines = load_file("/tmp/nixos-rebuild.log")?;
    let warnings = extract_evaluation_warnings(lines)?;

    let mut output = File::create("/tmp/nixos-rebuild-warnings.txt")?;

    for warning in warnings.iter().filter_map(|s| Warning::from_line(s)) {
        let line_nr = warning.line.unwrap_or(1);
        let line = format!(
            "{}:{}:1: warning: {} -> {}\n",
            warning.file, line_nr, warning.old_option, warning.new_option
        );

        output.write_all(line.as_bytes())?;
    }

    println!("Warnings written to /tmp/nixos-rebuild-warnings.txt");

    Ok(())
}

fn load_file(path: &str) -> Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut lines = Vec::new();

    for line in reader.lines() {
        lines.push(line?);
    }

    Ok(lines)
}

fn extract_evaluation_warnings(lines: Vec<String>) -> Result<Vec<String>> {
    Ok(lines
        .into_iter()
        .filter(|line| line.starts_with("evaluation warning"))
        .collect())
}

fn find_line_number(file_path: &str, search_text: &str) -> Option<usize> {
    //println!("Searching in: {}", file_path);
    //println!("Looking for: {}", search_text);

    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            println!("Failed to open file: {}", e);
            return None;
        }
    };

    let reader = BufReader::new(file);

    for (line_nr, line) in reader.lines().enumerate() {
        if let Ok(line) = line {
            if line.contains(search_text) {
                //println!("Found on line {}", line_nr + 1);
                return Some(line_nr + 1);
            }
        }
    }

    //println!("Not found in file");
    None
}

struct Warning {
    message: String,
    file: String,
    line: Option<usize>,
    old_option: String,
    new_option: String,
}

impl Warning {
    fn from_line(line: &str) -> Option<Self> {
        //println!("{}", line);

        let message = line.strip_prefix("evaluation warning: ")?.to_string();

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

        let full_file_path = format!("/home/xam/nixos/{}", file);
        let search_term = old_option.split('.').last()?;
        let line_nr = find_line_number(&full_file_path, &search_term);

        //println!(
        //    "file: {file}, old option: {old_option}, new option: {new_option}, linenr: {:?}",
        //    line_nr
        //);

        Some(Warning {
            message,
            file: full_file_path,
            line: line_nr,
            old_option,
            new_option,
        })
    }
}
