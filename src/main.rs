use std::env;
use pest::Parser;
use pest_derive::Parser;
use walkdir::{DirEntry, WalkDir};

mod ast;
mod parser;
mod interpreter;

use ast::*;
use parser::*;
use crate::interpreter::Interpreter;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct FindCommandParser;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let cmdline = args.join(" ");
    
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
