use clap::Parser;
use walkdir::WalkDir;
use std::fs;
use std::path::Path;
use std::io::Write;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::stdin;
use std::io::stdout;
use std::io::stderr;    

#[derive(Debug, Parser)]
#[command(author, version, about)]
/// Rust version of `find`
struct Args {
    /// List of starting points
    #[arg(value_name = "DIRS", default_value = ".")]
    dirs: Vec<String>,
}



fn run(mut _args: Args) -> anyhow::Result<()> {
    for dir in _args.dirs {
        for entry in WalkDir::new(dir) {
            let entry = entry?;
            println!("{}", entry.path().display());
        }
    }
    Ok(())
}
fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
