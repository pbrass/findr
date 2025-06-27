use pest::iterators::{Pair, Pairs};
use crate::ast::*;
use crate::{FindCommandParser, Rule};

/// Parser error type
#[derive(Debug)]
pub enum ParseError {
    UnexpectedRule { expected: String, found: String },
    InvalidNumber(String),
    InvalidSizeSpec(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedRule { expected, found } => {
                write!(f, "Expected {}, found {}", expected, found)
            }
            ParseError::InvalidNumber(s) => write!(f, "Invalid number: {}", s),
            ParseError::InvalidSizeSpec(s) => write!(f, "Invalid size specification: {}", s),
        }
    }
}

impl std::error::Error for ParseError {}

/// Converts a pest parse tree to our AST
pub fn parse_to_ast(pairs: Pairs<Rule>) -> Result<Expr, ParseError> {
    for pair in pairs {
        match pair.as_rule() {
            Rule::Program => {
                let mut inner = pair.into_inner();
                if let Some(expr_pair) = inner.next() {
                    return parse_expr(expr_pair);
                }
            }
            Rule::Expr => {
                return parse_expr(pair);
            }
            _ => continue,
        }
    }
    Err(ParseError::UnexpectedRule {
        expected: "Program".to_string(),
        found: "None".to_string(),
    })
}

fn parse_expr(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    match pair.as_rule() {
        Rule::Expr => {
            let inner = pair.into_inner().next().unwrap();
            parse_expr(inner)
        }
        Rule::UnaryExpr => parse_unary_expr(pair),
        Rule::BinaryExpr => parse_binary_expr(pair),
        Rule::Term => parse_term(pair),
        _ => Err(ParseError::UnexpectedRule {
            expected: "Expr".to_string(),
            found: format!("{:?}", pair.as_rule()),
        }),
    }
}

fn parse_unary_expr(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut inner = pair.into_inner();
    let _not = inner.next().unwrap(); // Skip the Not token
    let term = inner.next().unwrap();
    let expr = parse_term(term)?;
    Ok(Expr::Not(Box::new(expr)))
}

fn parse_binary_expr(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::AndExpr => parse_and_expr(inner),
        Rule::OrExpr => parse_or_expr(inner),
        _ => Err(ParseError::UnexpectedRule {
            expected: "AndExpr or OrExpr".to_string(),
            found: format!("{:?}", inner.as_rule()),
        }),
    }
}

fn parse_and_expr(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut inner = pair.into_inner();
    let first_term = inner.next().unwrap();
    let mut left = parse_term(first_term)?;
    
    while let Some(next_pair) = inner.next() {
        match next_pair.as_rule() {
            Rule::AndOperator => {
                // Skip the operator, get the next expression
                if let Some(expr_pair) = inner.next() {
                    let right = parse_expr(expr_pair)?;
                    left = Expr::And(Box::new(left), Box::new(right));
                }
            }
            Rule::Expr => {
                // Implicit AND (no operator)
                let right = parse_expr(next_pair)?;
                left = Expr::And(Box::new(left), Box::new(right));
            }
            _ => {}
        }
    }
    
    Ok(left)
}

fn parse_or_expr(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    let mut inner = pair.into_inner();
    let first_term = inner.next().unwrap();
    let mut left = parse_term(first_term)?;
    
    while let Some(next_pair) = inner.next() {
        match next_pair.as_rule() {
            Rule::OrOperator => {
                // Skip the operator, get the next expression
                if let Some(expr_pair) = inner.next() {
                    let right = parse_expr(expr_pair)?;
                    left = Expr::Or(Box::new(left), Box::new(right));
                }
            }
            _ => {}
        }
    }
    
    Ok(left)
}

fn parse_term(pair: Pair<Rule>) -> Result<Expr, ParseError> {
    match pair.as_rule() {
        Rule::Term => {
            let inner = pair.into_inner().next().unwrap();
            match inner.as_rule() {
                Rule::Test => {
                    let test = parse_test(inner)?;
                    Ok(Expr::Test(test))
                }
                Rule::Expr => {
                    // Parenthesized expression
                    parse_expr(inner)
                }
                _ => Err(ParseError::UnexpectedRule {
                    expected: "Test or Expr".to_string(),
                    found: format!("{:?}", inner.as_rule()),
                }),
            }
        }
        _ => Err(ParseError::UnexpectedRule {
            expected: "Term".to_string(),
            found: format!("{:?}", pair.as_rule()),
        }),
    }
}

fn parse_test(pair: Pair<Rule>) -> Result<Test, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::Path => {
            let mut inner = inner.into_inner();
            //let _name_token = inner.next().unwrap(); // Skip "-path"
            let glob = inner.next().unwrap();
            Ok(Test::Path(glob.as_str().to_string()))
        }
        Rule::Ipath => {
            let mut inner = inner.into_inner();
            let glob = inner.next().unwrap();
            Ok(Test::Ipath(glob.as_str().to_string()))
        }
        Rule::Name => {
            let mut inner = inner.into_inner();
            //let _name_token = inner.next().unwrap(); // Skip "-name"
            let glob = inner.next().unwrap();
            Ok(Test::Name(glob.as_str().to_string()))
        }
        Rule::Iname => {
            let mut inner = inner.into_inner();
            //let _iname_token = inner.next().unwrap(); // Skip "-iname"
            let glob = inner.next().unwrap();
            Ok(Test::Iname(glob.as_str().to_string()))
        }
        Rule::Regex => {
            let mut inner = inner.into_inner();
            //let _regex_token = inner.next().unwrap(); // Skip "-regex"
            let pattern = inner.next().unwrap();
            Ok(Test::Regex(pattern.as_str().to_string()))
        }
        Rule::Iregex => {
            let mut inner = inner.into_inner();
            let pattern = inner.next().unwrap();
            Ok(Test::Iregex(pattern.as_str().to_string()))
        }
        Rule::True => Ok(Test::True),
        Rule::False => Ok(Test::False),
        Rule::Type => {
            let mut inner = inner.into_inner();
            //let _type_token = inner.next().unwrap(); // Skip "-type"
            let filetype = inner.next().unwrap();
            let file_type = parse_filetype(filetype)?;
            Ok(Test::Type(file_type))
        }
        Rule::Size => {
            let mut inner = inner.into_inner();
            //let _size_token = inner.next().unwrap(); // Skip "-size"
            let sizespec = inner.next().unwrap();
            let size_spec = parse_sizespec(sizespec)?;
            Ok(Test::Size(size_spec))
        }
        Rule::Empty => {
            Ok(Test::Empty)
        }
        Rule::Amin => {
            let mut inner = inner.into_inner();
            let timespec = inner.next().unwrap();
            let time_spec = parse_timespec(timespec)?;
            Ok(Test::Amin(time_spec))
        }
        Rule::Atime => {
            let mut inner = inner.into_inner();
            let timespec = inner.next().unwrap();
            let time_spec = parse_timespec(timespec)?;
            Ok(Test::Atime(time_spec))
        }
        Rule::Ctime => {
            let mut inner = inner.into_inner();
            let timespec = inner.next().unwrap();
            let time_spec = parse_timespec(timespec)?;
            Ok(Test::Ctime(time_spec))
        }
        Rule::Cmin => {
            let mut inner = inner.into_inner();
            let timespec = inner.next().unwrap();
            let time_spec = parse_timespec(timespec)?;
            Ok(Test::Cmin(time_spec))
        }
        Rule::Mmin => {
            let mut inner = inner.into_inner();
            let timespec = inner.next().unwrap();
            let time_spec = parse_timespec(timespec)?;
            Ok(Test::Mmin(time_spec))
        }
        Rule::Mtime => {
            let mut inner = inner.into_inner();
            let timespec = inner.next().unwrap();
            let time_spec = parse_timespec(timespec)?;
            Ok(Test::Mtime(time_spec))
        }
        Rule::Anewer => {
            let mut inner = inner.into_inner();
            let filepath = inner.next().unwrap();
            Ok(Test::Anewer(filepath.as_str().to_string()))
        }
        Rule::Cnewer => {
            let mut inner = inner.into_inner();
            let filepath = inner.next().unwrap();
            Ok(Test::Cnewer(filepath.as_str().to_string()))
        }
        Rule::Mnewer => {
            let mut inner = inner.into_inner();
            let filepath = inner.next().unwrap();
            Ok(Test::Mnewer(filepath.as_str().to_string()))
        }
        Rule::Newer => {
            let mut inner = inner.into_inner();
            let filepath = inner.next().unwrap();
            Ok(Test::Newer(filepath.as_str().to_string()))
        }
        Rule::User => {
            let mut inner = inner.into_inner();
            let username = inner.next().unwrap();
            Ok(Test::User(username.as_str().to_string()))
        }
        Rule::Group => {
            let mut inner = inner.into_inner();
            let groupname = inner.next().unwrap();
            Ok(Test::Group(groupname.as_str().to_string()))
        }
        Rule::Uid => {
            let mut inner = inner.into_inner();
            let uid_str = inner.next().unwrap();
            let uid = uid_str.as_str().parse::<u32>()
                .map_err(|_| ParseError::InvalidNumber(uid_str.as_str().to_string()))?;
            Ok(Test::Uid(uid))
        }
        Rule::Gid => {
            let mut inner = inner.into_inner();
            let gid_str = inner.next().unwrap();
            let gid = gid_str.as_str().parse::<u32>()
                .map_err(|_| ParseError::InvalidNumber(gid_str.as_str().to_string()))?;
            Ok(Test::Gid(gid))
        }
        _ => Err(ParseError::UnexpectedRule {
            expected: "Test variant".to_string(),
            found: format!("{:?}", inner.as_rule()),
        }),
    }
}

fn parse_filetype(pair: Pair<Rule>) -> Result<FileType, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::BlockFile => Ok(FileType::BlockFile),
        Rule::CharFile => Ok(FileType::CharFile),
        Rule::Directory => Ok(FileType::Directory),
        Rule::NamedPipe => Ok(FileType::NamedPipe),
        Rule::RegularFile => Ok(FileType::RegularFile),
        Rule::SymbolicLink => Ok(FileType::SymbolicLink),
        Rule::Socket => Ok(FileType::Socket),
        _ => Err(ParseError::UnexpectedRule {
            expected: "FileType".to_string(),
            found: format!("{:?}", inner.as_rule()),
        }),
    }
}

fn parse_sizespec(pair: Pair<Rule>) -> Result<SizeSpec, ParseError> {
    let mut sign = Sign::None;
    let mut value = 0u64;
    let mut suffix = None;
    
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::Sign => {
                let sign_inner = inner.into_inner().next().unwrap();
                sign = match sign_inner.as_rule() {
                    Rule::Plus => Sign::Plus,
                    Rule::Minus => Sign::Minus,
                    _ => Sign::None,
                };
            }
            Rule::Number => {
                value = inner.as_str().parse::<u64>()
                    .map_err(|_| ParseError::InvalidNumber(inner.as_str().to_string()))?;
            }
            Rule::SizeSuffix => {
                let suffix_inner = inner.into_inner().next().unwrap();
                suffix = Some(match suffix_inner.as_rule() {
                    Rule::Blocks => SizeSuffix::Blocks,
                    Rule::Bytes => SizeSuffix::Bytes,
                    Rule::Words => SizeSuffix::Words,
                    Rule::Kb => SizeSuffix::Kb,
                    Rule::Mb => SizeSuffix::Mb,
                    Rule::Gb => SizeSuffix::Gb,
                    _ => return Err(ParseError::UnexpectedRule {
                        expected: "SizeSuffix".to_string(),
                        found: format!("{:?}", suffix_inner.as_rule()),
                    }),
                });
            }
            _ => {}
        }
    }
    
    Ok(SizeSpec { sign, value, suffix })
}

fn parse_timespec(pair: Pair<Rule>) -> Result<TimeSpec, ParseError> {
    let mut sign = Sign::None;
    let mut value = 0u64;
    
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::Sign => {
                let sign_inner = inner.into_inner().next().unwrap();
                sign = match sign_inner.as_rule() {
                    Rule::Plus => Sign::Plus,
                    Rule::Minus => Sign::Minus,
                    _ => Sign::None,
                };
            }
            Rule::Number => {
                value = inner.as_str().parse::<u64>()
                    .map_err(|_| ParseError::InvalidNumber(inner.as_str().to_string()))?;
            }
            _ => {}
        }
    }
    
    Ok(TimeSpec { sign, value })
}