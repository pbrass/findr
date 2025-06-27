use std::fmt;

/// The AST for the find command parser
#[derive(Debug, Clone)]
pub enum Expr {
    /// A unary expression with a not operator
    Not(Box<Expr>),
    /// A binary AND expression
    And(Box<Expr>, Box<Expr>),
    /// A binary OR expression
    Or(Box<Expr>, Box<Expr>),
    /// A test expression
    Test(Test),
}

/// Test expressions for the find command
#[derive(Debug, Clone)]
pub enum Test {
    /// Match paths by name (case-sensitive)
    Path(String),
    /// Match files by name (case-sensitive)
    Name(String),
    /// Match files by name (case-insensitive)
    Iname(String),
    /// Match files by regex pattern
    Regex(String),
    /// Always evaluates to true
    True,
    /// Always evaluates to false
    False,
    /// Match files by type
    Type(FileType),
    /// Match files by size
    Size(SizeSpec),
    /// Match files by size
    Empty,
    /// Match files by access time in minutes
    Amin(TimeSpec),
    /// Match files by access time in days
    Atime(TimeSpec),
    /// Match files by creation time in days
    Ctime(TimeSpec),
    /// Match files by creation time in minutes
    Cmin(TimeSpec),
    /// Match files by modification time in minutes
    Mmin(TimeSpec),
    /// Match files by modification time in days
    Mtime(TimeSpec),
    /// Match files accessed more recently than the reference file
    Anewer(String),
    /// Match files created more recently than the reference file
    Cnewer(String),
    /// Match files modified more recently than the reference file
    Mnewer(String),
    /// Match files modified more recently than the reference file (alias for Mnewer)
    Newer(String),
    /// Match paths by glob pattern (case-insensitive)
    Ipath(String),
    /// Match files by regex pattern (case-insensitive)
    Iregex(String),
    /// Match files by owner username or UID
    User(String),
    /// Match files by group name or GID
    Group(String),
    /// Match files by numeric UID
    Uid(u32),
    /// Match files by numeric GID
    Gid(u32),
}

/// File types for the -type test
#[derive(Debug, Clone)]
pub enum FileType {
    /// Block special file
    BlockFile,
    /// Character special file
    CharFile,
    /// Directory
    Directory,
    /// Named pipe (FIFO)
    NamedPipe,
    /// Regular file
    RegularFile,
    /// Symbolic link
    SymbolicLink,
    /// Socket
    Socket,
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileType::BlockFile => write!(f, "b"),
            FileType::CharFile => write!(f, "c"),
            FileType::Directory => write!(f, "d"),
            FileType::NamedPipe => write!(f, "p"),
            FileType::RegularFile => write!(f, "f"),
            FileType::SymbolicLink => write!(f, "l"),
            FileType::Socket => write!(f, "s"),
        }
    }
}

/// Sign for size specifications
#[derive(Debug, Clone)]
pub enum Sign {
    /// Exactly this size
    None,
    /// Greater than this size
    Plus,
    /// Less than this size
    Minus,
}

/// Size suffix for size specifications
#[derive(Debug, Clone)]
pub enum SizeSuffix {
    /// 512-byte blocks (default)
    Blocks,
    /// Bytes
    Bytes,
    /// 2-byte words
    Words,
    /// Kilobytes (1024 bytes)
    Kb,
    /// Megabytes (1024 * 1024 bytes)
    Mb,
    /// Gigabytes (1024 * 1024 * 1024 bytes)
    Gb,
}

impl fmt::Display for SizeSuffix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SizeSuffix::Blocks => write!(f, "b"),
            SizeSuffix::Bytes => write!(f, "c"),
            SizeSuffix::Words => write!(f, "w"),
            SizeSuffix::Kb => write!(f, "k"),
            SizeSuffix::Mb => write!(f, "M"),
            SizeSuffix::Gb => write!(f, "G"),
        }
    }
}

/// Size specification for the -size test
#[derive(Debug, Clone)]
pub struct SizeSpec {
    /// Sign (none, +, -)
    pub sign: Sign,
    /// Size value
    pub value: u64,
    /// Size suffix
    pub suffix: Option<SizeSuffix>,
}

/// Time specification for time-based tests (like -amin, -atime, -ctime, -cmin, -mmin, -mtime)
#[derive(Debug, Clone)]
pub struct TimeSpec {
    /// Sign (none, +, -)
    pub sign: Sign,
    /// Time value (units depend on the test: minutes for -amin/-cmin/-mmin, days for -atime/-ctime/-mtime)
    pub value: u64,
}