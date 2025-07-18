use std::env;
use std::process;
use regex::Regex;
use pest::Parser;
use pest_derive::Parser;
use walkdir::WalkDir;

mod ast;
mod parser;
mod interpreter;

use parser::*;
use crate::interpreter::Interpreter;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct FindCommandParser;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let cmdline = args.join(" ");
    let help_re = Regex::new(r"(?x)(?:^|\s)(-h|--help)(?:\s|$)").expect("Failed to compile help regex");
    if help_re.is_match(&cmdline) {
        println!("Usage: findr [paths] -expression\nAvailable expressions:");
        println!("  -true             always true");
        println!("  -false            always false");
        println!("  -expr -and -expr  boolean and");
        println!("  -expr -or -expr   boolean or");
        println!("  -not -expr        boolean not");
        println!("  -path <glob>      Match whole path");
        println!("  -ipath <glob>     Match whole path, case insensitive");
        println!("  -name <glob>      Match filename");
        println!("  -iname <glob>     Match filename, case insensitive");
        println!("  -regex <re>       Regex match filename");
        println!("  -iregex <re>      Regex match filename, case insensitive");
        println!("  -type <type>      Match type - f for file, d for dir, etc");
        println!("  -user <user>      Match files owned by username");
        println!("  -group <group>    Match files with groupname");
        println!("  -uid <uid>        Match files owned by uid");
        println!("  -gid <gid>        Match files with group id gid");
        println!("  -perm <perm>      Match files with specified permissions");
        println!("  -atime <time>     Match files by access time");
        println!("  -amin <time>      Match files by access time in minutes");
        println!("  -anewer <other>   Match files accessed more recently than other file");
        println!("  -ctime <time>     Match files by create time");
        println!("  -cmin <time>      Match files by create time in minutes");
        println!("  -cnewer <other>   Match files created more recently than other file");
        println!("  -mtime <time>     Match files by modification time");
        println!("  -mmin <time>      Match files by modification time in minutes");
        println!("  -mnewer <other>   Match files modified more recently than other file");
        println!("Supported expressions should work just like they do in GNU find, consult their documentation for more details (man find)");
        process::exit(0);
    }
    
    // Find the first occurrence of either '-', '(', or '!'
    let split_pos = cmdline.find('-').unwrap_or(cmdline.len())
        .min(cmdline.find('(').unwrap_or(cmdline.len()))
        .min(cmdline.find('!').unwrap_or(cmdline.len()));
    
    let (dirstr, expr) = if split_pos < cmdline.len() {
        cmdline.split_at(split_pos)
    } else {
        (cmdline.as_str(),"")
    };
    
    //println!("Parsing command: {cmdline}");
    let mut dirs: Vec<String> = dirstr.split(" ").into_iter()
        .map(|d| d.to_string())
        .filter(|d| !d.is_empty())
        .collect();
    if dirs.len() == 0 {
        dirs.push(".".to_string());
    }
    
    let mut expr = expr.trim().to_string();
    if expr.len() == 0 {expr = "-true".to_string();}

    let parsed = FindCommandParser::parse(Rule::Program, &expr)
        .expect("Failed to parse command line");

    match parse_to_ast(parsed) {
        Ok(ast) => {
            for dir in dirs {
                for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                    if Interpreter::evaluate(&ast, &entry) {
                        println!("{}", entry.path().display());
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to build AST: {}", e);
        }
    }
}
