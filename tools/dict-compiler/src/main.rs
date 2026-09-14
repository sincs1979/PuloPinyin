//! Compile `system.dict` / `user.dict` from TSV or learned.db.

use engine::dictionary::{parse_tsv, BinaryDict};
use engine::learning::{compile_user_dict, LearnedDb};
use engine::ranking::score::now_unix;
use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let mut system_tsv: Option<PathBuf> = None;
    let mut learned_db: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut mode = "system";

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--system" => {
                mode = "system";
                system_tsv = args.next().map(PathBuf::from);
            }
            "--user" => {
                mode = "user";
                learned_db = args.next().map(PathBuf::from);
            }
            "-o" | "--output" => output = args.next().map(PathBuf::from),
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown argument: {other}");
                print_help();
                return ExitCode::FAILURE;
            }
        }
    }

    let output = match output {
        Some(p) => p,
        None => {
            eprintln!("missing -o output path");
            print_help();
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = run(mode, system_tsv.as_deref(), learned_db.as_deref(), &output) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run(
    mode: &str,
    system_tsv: Option<&Path>,
    learned_db: Option<&Path>,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(dir) = output.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    match mode {
        "system" => {
            let path = system_tsv.ok_or("missing --system <tsv>")?;
            let text = std::fs::read_to_string(path)?;
            let entries = parse_tsv(&text)?;
            let dict = BinaryDict::from_entries(entries);
            dict.write_to(output)?;
            println!(
                "wrote {} entries → {}",
                dict.entries.len(),
                output.display()
            );
        }
        "user" => {
            let path = learned_db.ok_or("missing --user <learned.db>")?;
            let db = LearnedDb::open(path)?;
            let dict = compile_user_dict(&db, output, now_unix())?;
            println!(
                "compiled {} learned entries → {}",
                dict.entries.len(),
                output.display()
            );
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "\
dict-compiler — 部落输入法 dictionary builder

  dict-compiler --system data/system.tsv -o resources/system.dict
  dict-compiler --user  ~/Library/Application\\ Support/部落输入法/learned.db \\
                -o ~/Library/Application\\ Support/部落输入法/user.dict
"
    );
}
