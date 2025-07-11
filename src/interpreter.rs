use walkdir::DirEntry;
use std::time::SystemTime;
use std::fs;
use regex::Regex;
use glob::Pattern;
use crate::ast::*;
//use libc;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use file_owner::PathExt;

/// Interpreter for evaluating AST expressions against directory entries
pub struct Interpreter;

impl Interpreter {
    /// Evaluates an AST expression against a directory entry
    pub fn evaluate(expr: &Expr, entry: &DirEntry) -> bool {
        match expr {
            Expr::Not(inner) => !Self::evaluate(inner, entry),
            Expr::And(left, right) => Self::evaluate(left, entry) && Self::evaluate(right, entry),
            Expr::Or(left, right) => Self::evaluate(left, entry) || Self::evaluate(right, entry),
            Expr::Test(test) => Self::evaluate_test(test, entry),
        }
    }

    fn evaluate_test(test: &Test, entry: &DirEntry) -> bool {
        match test {
            Test::Path(pattern) => Self::match_path(pattern, entry, false),
            Test::Name(pattern) => Self::match_name(pattern, entry, false),
            Test::Iname(pattern) => Self::match_name(pattern, entry, true),
            Test::Regex(pattern) => Self::match_regex(pattern, entry),
            Test::True => true,
            Test::False => false,
            Test::Type(file_type) => Self::match_type(file_type, entry),
            Test::Size(size_spec) => Self::match_size(size_spec, entry),
            Test::Empty => Self::match_empty(entry),
            Test::Amin(time_spec) => Self::match_amin(time_spec, entry),
            Test::Atime(time_spec) => Self::match_atime(time_spec, entry),
            Test::Ctime(time_spec) => Self::match_ctime(time_spec, entry),
            Test::Cmin(time_spec) => Self::match_cmin(time_spec, entry),
            Test::Mmin(time_spec) => Self::match_mmin(time_spec, entry),
            Test::Mtime(time_spec) => Self::match_mtime(time_spec, entry),
            Test::Anewer(filepath) => Self::match_anewer(filepath, entry),
            Test::Cnewer(filepath) => Self::match_cnewer(filepath, entry),
            Test::Mnewer(filepath) => Self::match_mnewer(filepath, entry),
            Test::Newer(filepath) => Self::match_newer(filepath, entry),
            Test::Ipath(pattern) => Self::match_path(pattern, entry, true),
            Test::Iregex(pattern) => Self::match_iregex(pattern, entry),
            Test::User(username) => Self::match_user(username, entry),
            Test::Group(groupname) => Self::match_group(groupname, entry),
            Test::Uid(uid) => Self::match_uid(*uid, entry),
            Test::Gid(gid) => Self::match_gid(*gid, entry),
            Test::Perm(perm_spec) => Self::match_perm(perm_spec, entry),
        }
    }

    // Helper function for glob pattern matching
    fn match_glob_pattern(pattern: &str, target: &str, case_insensitive: bool) -> bool {
        match Pattern::new(pattern) {
            Ok(glob_pattern) => {
                if case_insensitive {
                    glob_pattern.matches(&target.to_lowercase())
                } else {
                    glob_pattern.matches(target)
                }
            }
            Err(_) => false,
        }
    }

    // Helper function for getting file metadata
    fn get_metadata(entry: &DirEntry) -> Option<std::fs::Metadata> {
        entry.metadata().ok()
    }

    // Helper function for time-based comparisons
    fn compare_time_spec(file_time: SystemTime, time_spec: &TimeSpec, time_unit_seconds: u64) -> bool {
        let now = SystemTime::now();
        let time_diff = match now.duration_since(file_time) {
            Ok(duration) => duration,
            Err(_) => return false, // File time in the future
        };

        let time_ago = time_diff.as_secs() / time_unit_seconds;
        let target_time = time_spec.value;

        match time_spec.sign {
            Sign::None => time_ago == target_time,
            Sign::Plus => time_ago > target_time,
            Sign::Minus => time_ago < target_time,
        }
    }

    // Helper function for newer-style comparisons
    fn compare_file_times<F>(entry: &DirEntry, filepath: &str, time_getter: F) -> bool 
    where
        F: Fn(&std::fs::Metadata) -> Result<SystemTime, std::io::Error>,
    {
        let entry_metadata = match Self::get_metadata(entry) {
            Some(metadata) => metadata,
            None => return false,
        };
        let entry_time = match time_getter(&entry_metadata) {
            Ok(time) => time,
            Err(_) => return false,
        };

        let reference_metadata = match fs::metadata(filepath) {
            Ok(metadata) => metadata,
            Err(_) => return false,
        };
        let reference_time = match time_getter(&reference_metadata) {
            Ok(time) => time,
            Err(_) => return false,
        };

        entry_time > reference_time
    }

    fn match_path(pattern: &str, entry: &DirEntry, case_insensitive: bool) -> bool {
        let file_name = entry.path().to_string_lossy();
        Self::match_glob_pattern(pattern, &file_name, case_insensitive)
    }

    fn match_name(pattern: &str, entry: &DirEntry, case_insensitive: bool) -> bool {
        let file_name = entry.file_name().to_string_lossy();
        Self::match_glob_pattern(pattern, &file_name, case_insensitive)
    }

    fn match_regex(pattern: &str, entry: &DirEntry) -> bool {
        let anchored_pattern = format!("^{}$", pattern);
        match Regex::new(&anchored_pattern) {
            Ok(regex) => {
                let path_str = entry.path().to_string_lossy();
                regex.is_match(&path_str)
            }
            Err(_) => false,
        }
    }

    fn match_iregex(pattern: &str, entry: &DirEntry) -> bool {
        // Create case-insensitive regex by prefixing with (?i)
        let case_insensitive_pattern = format!("(?i)^{}$", pattern);
        match Regex::new(&case_insensitive_pattern) {
            Ok(regex) => {
                let path_str = entry.path().to_string_lossy();
                regex.is_match(&path_str)
            }
            Err(_) => false,
        }
    }

    fn match_type(file_type: &FileType, entry: &DirEntry) -> bool {
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => return false,
        };

        match file_type {
            FileType::RegularFile => metadata.is_file(),
            FileType::Directory => metadata.is_dir(),
            FileType::SymbolicLink => metadata.file_type().is_symlink(),
            FileType::BlockFile => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::FileTypeExt;
                    metadata.file_type().is_block_device()
                }
                #[cfg(not(unix))]
                false
            }
            FileType::CharFile => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::FileTypeExt;
                    metadata.file_type().is_char_device()
                }
                #[cfg(not(unix))]
                false
            }
            FileType::NamedPipe => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::FileTypeExt;
                    metadata.file_type().is_fifo()
                }
                #[cfg(not(unix))]
                false
            }
            FileType::Socket => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::FileTypeExt;
                    metadata.file_type().is_socket()
                }
                #[cfg(not(unix))]
                false
            }
        }
    }

    fn match_size(size_spec: &SizeSpec, entry: &DirEntry) -> bool {
        let metadata = match Self::get_metadata(entry) {
            Some(metadata) => metadata,
            None => return false,
        };

        let file_size = metadata.len();
        let target_size = Self::calculate_size_in_bytes(size_spec);

        match size_spec.sign {
            Sign::None => file_size == target_size,
            Sign::Plus => file_size > target_size,
            Sign::Minus => file_size < target_size,
        }
    }
    
    fn match_empty(entry: &DirEntry) -> bool {
        let metadata = match Self::get_metadata(entry) {
            Some(metadata) => metadata,
            None => return false,
        };

        let file_size = metadata.len();
        return file_size == 0;
    }

    fn match_amin(time_spec: &TimeSpec, entry: &DirEntry) -> bool {
        let metadata = match Self::get_metadata(entry) {
            Some(metadata) => metadata,
            None => return false,
        };
        let accessed_time = match metadata.accessed() {
            Ok(time) => time,
            Err(_) => return false,
        };
        Self::compare_time_spec(accessed_time, time_spec, 60)
    }

    fn match_atime(time_spec: &TimeSpec, entry: &DirEntry) -> bool {
        let metadata = match Self::get_metadata(entry) {
            Some(metadata) => metadata,
            None => return false,
        };
        let accessed_time = match metadata.accessed() {
            Ok(time) => time,
            Err(_) => return false,
        };
        Self::compare_time_spec(accessed_time, time_spec, 24 * 60 * 60)
    }

    fn match_ctime(time_spec: &TimeSpec, entry: &DirEntry) -> bool {
        let metadata = match Self::get_metadata(entry) {
            Some(metadata) => metadata,
            None => return false,
        };
        let created_time = match metadata.created() {
            Ok(time) => time,
            Err(_) => return false,
        };
        Self::compare_time_spec(created_time, time_spec, 24 * 60 * 60)
    }

    fn match_cmin(time_spec: &TimeSpec, entry: &DirEntry) -> bool {
        let metadata = match Self::get_metadata(entry) {
            Some(metadata) => metadata,
            None => return false,
        };
        let created_time = match metadata.created() {
            Ok(time) => time,
            Err(_) => return false,
        };
        Self::compare_time_spec(created_time, time_spec, 60)
    }

    fn match_mmin(time_spec: &TimeSpec, entry: &DirEntry) -> bool {
        let metadata = match Self::get_metadata(entry) {
            Some(metadata) => metadata,
            None => return false,
        };
        let modified_time = match metadata.modified() {
            Ok(time) => time,
            Err(_) => return false,
        };
        Self::compare_time_spec(modified_time, time_spec, 60)
    }

    fn match_mtime(time_spec: &TimeSpec, entry: &DirEntry) -> bool {
        let metadata = match Self::get_metadata(entry) {
            Some(metadata) => metadata,
            None => return false,
        };
        let modified_time = match metadata.modified() {
            Ok(time) => time,
            Err(_) => return false,
        };
        Self::compare_time_spec(modified_time, time_spec, 24 * 60 * 60)
    }

    fn match_anewer(filepath: &str, entry: &DirEntry) -> bool {
        Self::compare_file_times(entry, filepath, |metadata| metadata.accessed())
    }

    fn match_cnewer(filepath: &str, entry: &DirEntry) -> bool {
        Self::compare_file_times(entry, filepath, |metadata| metadata.created())
    }

    fn match_mnewer(filepath: &str, entry: &DirEntry) -> bool {
        Self::compare_file_times(entry, filepath, |metadata| metadata.modified())
    }

    fn match_newer(filepath: &str, entry: &DirEntry) -> bool {
        // -newer is an alias for -mnewer (modification time comparison)
        Self::match_mnewer(filepath, entry)
    }

    fn match_user(username: &str, entry: &DirEntry) -> bool {
        let metadata = match Self::get_metadata(entry) {
            Some(metadata) => metadata,
            None => return false,
        };

        #[cfg(unix)]
        {
            let file_uid = metadata.uid();
            
            // Try to parse as numeric UID first
            if let Ok(target_uid) = username.parse::<u32>() {
                return file_uid == target_uid;
            }
            
            // If not numeric, try to resolve username to UID
            // This is a simplified implementation - in practice you'd use getpwnam
            // For now, we'll just do string comparison with the current user
            let o = entry.path().owner().unwrap();
            return username == o.name().unwrap().unwrap().to_string();
/*            if let Ok(current_user) = std::env::var("USER") {
                if username == current_user {
                    // Get current user's UID
                    return file_uid == unsafe { libc::getuid() };
                }
            }

            false

 */
        }
        
        #[cfg(not(unix))]
        {
            // On non-Unix systems, we can't easily check file ownership
            // Return false or implement Windows-specific logic
            false
        }
    }

    fn match_group(groupname: &str, entry: &DirEntry) -> bool {
        let metadata = match Self::get_metadata(entry) {
            Some(metadata) => metadata,
            None => return false,
        };

        #[cfg(unix)]
        {
            let file_gid = metadata.gid();
            
            // Try to parse as numeric GID first
            if let Ok(target_gid) = groupname.parse::<u32>() {
                return file_gid == target_gid;
            }
            
            // If not numeric, try to resolve group name to GID
            // This is a simplified implementation - in practice you'd use getgrnam
            // For now, we'll just do string comparison with the current group
            let g = entry.path().owner().unwrap();
            return groupname == g.name().unwrap().unwrap().to_string();
/*
            if let Ok(current_group) = std::env::var("GROUP") {
                if groupname == current_group {
                    // Get current user's primary GID
                    return file_gid == unsafe { libc::getgid() };
                }
            }
            
            false

 */
        }
        
        #[cfg(not(unix))]
        {
            // On non-Unix systems, we can't easily check file group ownership
            // Return false or implement Windows-specific logic
            false
        }
    }

    fn match_uid(target_uid: u32, entry: &DirEntry) -> bool {
        let metadata = match Self::get_metadata(entry) {
            Some(metadata) => metadata,
            None => return false,
        };

        #[cfg(unix)]
        {
            let file_uid = metadata.uid();
            file_uid == target_uid
        }
        
        #[cfg(not(unix))]
        {
            // On non-Unix systems, we can't easily check file ownership
            false
        }
    }

    fn match_gid(target_gid: u32, entry: &DirEntry) -> bool {
        let metadata = match Self::get_metadata(entry) {
            Some(metadata) => metadata,
            None => return false,
        };

        #[cfg(unix)]
        {
            let file_gid = metadata.gid();
            file_gid == target_gid
        }
        
        #[cfg(not(unix))]
        {
            // On non-Unix systems, we can't easily check file group ownership
            false
        }
    }


    fn match_perm(perm_spec: &PermSpec, entry: &DirEntry) -> bool {
        let metadata = match Self::get_metadata(entry) {
            Some(metadata) => metadata,
            None => return false,
        };

        #[cfg(unix)]
        {
            let file_mode = metadata.mode();
            let file_perms = file_mode & 0o777; // Extract permission bits
            
            match &perm_spec.term {
                PermTerm::Numeric(target_perms) => {
                    Self::match_numeric_perm(*target_perms, file_perms, &perm_spec.prefix)
                }
                PermTerm::Symbolic(statements) => {
                    Self::match_symbolic_perm(statements, file_perms, &perm_spec.prefix)
                }
            }
        }
        
        #[cfg(not(unix))]
        {
            // On non-Unix systems, permission checking is not supported
            false
        }
    }

    #[cfg(unix)]
    fn match_numeric_perm(target_perms: u32, file_perms: u32, prefix: &Option<PermPrefix>) -> bool {
        match prefix {
            None => {
                // Exact match
                file_perms == target_perms
            }
            Some(PermPrefix::AllMode) => {
                // All specified bits must be set (target_perms is a subset of file_perms)
                (file_perms & target_perms) == target_perms
            }
            Some(PermPrefix::AnyMode) => {
                // Any of the specified bits can be set
                (file_perms & target_perms) != 0
            }
        }
    }

    #[cfg(unix)]
    fn match_symbolic_perm(statements: &[SymPermStatement], file_perms: u32, prefix: &Option<PermPrefix>) -> bool {
        // Convert symbolic statements to numeric representation
        let target_perms = Self::symbolic_to_numeric(statements, file_perms);
        Self::match_numeric_perm(target_perms, file_perms, prefix)
    }

    #[cfg(unix)]
    fn symbolic_to_numeric(statements: &[SymPermStatement], current_perms: u32) -> u32 {
        let mut result_perms = current_perms;
        
        for statement in statements {
            let perm_mask = Self::get_permission_mask(&statement.principal, &statement.privileges);
            
            match statement.operator {
                SymPermOperator::Add => {
                    result_perms |= perm_mask;
                }
                SymPermOperator::Remove => {
                    result_perms &= !perm_mask;
                }
                SymPermOperator::Set => {
                    // Clear the relevant bits first, then set the new ones
                    let principal_mask = Self::get_principal_mask(&statement.principal);
                    result_perms &= !principal_mask;
                    result_perms |= perm_mask;
                }
            }
        }
        
        result_perms
    }

    #[cfg(unix)]
    fn get_permission_mask(principal: &SymPrincipal, privileges: &[SymPermPriv]) -> u32 {
        let mut mask = 0u32;
        
        for privilege in privileges {
            let perm_bit = match privilege {
                SymPermPriv::Read => 0o4,
                SymPermPriv::Write => 0o2,
                SymPermPriv::Execute => 0o1,
            };
            
            match principal {
                SymPrincipal::User => mask |= perm_bit << 6,
                SymPrincipal::Group => mask |= perm_bit << 3,
                SymPrincipal::Other => mask |= perm_bit,
                SymPrincipal::All => {
                    mask |= perm_bit << 6; // user
                    mask |= perm_bit << 3; // group
                    mask |= perm_bit;      // other
                }
            }
        }
        
        mask
    }

    #[cfg(unix)]
    fn get_principal_mask(principal: &SymPrincipal) -> u32 {
        match principal {
            SymPrincipal::User => 0o700,
            SymPrincipal::Group => 0o070,
            SymPrincipal::Other => 0o007,
            SymPrincipal::All => 0o777,
        }
    }

    fn calculate_size_in_bytes(size_spec: &SizeSpec) -> u64 {
        let multiplier = match size_spec.suffix.as_ref() {
            Some(SizeSuffix::Bytes) => 1,
            Some(SizeSuffix::Words) => 2,
            Some(SizeSuffix::Kb) => 1024,
            Some(SizeSuffix::Mb) => 1024 * 1024,
            Some(SizeSuffix::Gb) => 1024 * 1024 * 1024,
            Some(SizeSuffix::Blocks) | None => 512, // Default is 512-byte blocks
        };
        
        size_spec.value * multiplier
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_true_false() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();
        
        let entry = walkdir::WalkDir::new(&file_path)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        assert!(Interpreter::evaluate(&Expr::Test(Test::True), &entry));
        assert!(!Interpreter::evaluate(&Expr::Test(Test::False), &entry));
    }

    #[test]
    fn test_name_matching() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();
        
        let entry = walkdir::WalkDir::new(&file_path)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        let name_test = Test::Name("test.txt".to_string());
        assert!(Interpreter::evaluate(&Expr::Test(name_test), &entry));

        let wildcard_test = Test::Name("*.txt".to_string());
        assert!(Interpreter::evaluate(&Expr::Test(wildcard_test), &entry));

        let no_match_test = Test::Name("*.md".to_string());
        assert!(!Interpreter::evaluate(&Expr::Test(no_match_test), &entry));
    }

    #[test]
    fn test_type_matching() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();
        
        let file_entry = walkdir::WalkDir::new(&file_path)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        let dir_entry = walkdir::WalkDir::new(temp_dir.path())
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        let file_test = Test::Type(FileType::RegularFile);
        assert!(Interpreter::evaluate(&Expr::Test(file_test), &file_entry));
        
        let dir_test = Test::Type(FileType::Directory);
        assert!(Interpreter::evaluate(&Expr::Test(dir_test), &dir_entry));
    }

    #[test]
    fn test_boolean_logic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();
        
        let entry = walkdir::WalkDir::new(&file_path)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        // Test AND
        let and_expr = Expr::And(
            Box::new(Expr::Test(Test::True)),
            Box::new(Expr::Test(Test::Name("*.txt".to_string()))),
        );
        assert!(Interpreter::evaluate(&and_expr, &entry));

        // Test OR
        let or_expr = Expr::Or(
            Box::new(Expr::Test(Test::False)),
            Box::new(Expr::Test(Test::Name("*.txt".to_string()))),
        );
        assert!(Interpreter::evaluate(&or_expr, &entry));

        // Test NOT
        let not_expr = Expr::Not(Box::new(Expr::Test(Test::False)));
        assert!(Interpreter::evaluate(&not_expr, &entry));
    }

    #[test]
    #[cfg(unix)]
    fn test_numeric_permission_matching() {
        use std::os::unix::fs::PermissionsExt;
        
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();
        
        // Set specific permissions (644)
        let permissions = fs::Permissions::from_mode(0o644);
        fs::set_permissions(&file_path, permissions).unwrap();
        
        let entry = walkdir::WalkDir::new(&file_path)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        // Test exact match
        let perm_test = Test::Perm(PermSpec {
            prefix: None,
            term: PermTerm::Numeric(0o644),
        });
        assert!(Interpreter::evaluate(&Expr::Test(perm_test), &entry));

        // Test all mode (file has 644, checking for 044 should pass)
        let perm_test_all = Test::Perm(PermSpec {
            prefix: Some(PermPrefix::AllMode),
            term: PermTerm::Numeric(0o044),
        });
        assert!(Interpreter::evaluate(&Expr::Test(perm_test_all), &entry));

        // Test any mode (file has 644, checking for 200 should pass)
        let perm_test_any = Test::Perm(PermSpec {
            prefix: Some(PermPrefix::AnyMode),
            term: PermTerm::Numeric(0o200),
        });
        assert!(Interpreter::evaluate(&Expr::Test(perm_test_any), &entry));
    }

    #[test]
    #[cfg(unix)]
    fn test_symbolic_permission_matching() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();
        
        let entry = walkdir::WalkDir::new(&file_path)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        // Test symbolic permission (u+r should match files with user read permission)
        let symbolic_stmt = SymPermStatement {
            principal: SymPrincipal::User,
            operator: SymPermOperator::Add,
            privileges: vec![SymPermPriv::Read],
        };
        
        let perm_test = Test::Perm(PermSpec {
            prefix: Some(PermPrefix::AllMode),
            term: PermTerm::Symbolic(vec![symbolic_stmt]),
        });
        
        // This should typically pass since most files have user read permission
        assert!(Interpreter::evaluate(&Expr::Test(perm_test), &entry));
    }
}