use crate::models::InputError;
use std::path::Path;

/// Resolves the input argument to a raw equation string.
///
/// - If `arg` points to an existing file, reads and returns its contents.
/// - If the file exists but cannot be read, returns `InputError::FileReadError`.
/// - If `arg` does not point to an existing file, treats it as a raw equation string.
pub fn resolve_input(arg: &str) -> Result<String, InputError> {
    let path = Path::new(arg);

    if path.is_file() {
        // Path exists and is a regular file — read it
        std::fs::read_to_string(path).map_err(|err| {
            InputError::FileReadError(format!("cannot read file: {}: {}", arg, err))
        })
    } else if path.exists() {
        // Path exists but is not a regular file (e.g. a directory)
        Err(InputError::FileReadError(format!(
            "cannot read file: {}: not a regular file",
            arg
        )))
    } else {
        // Path does not exist — treat arg as a raw equation string
        Ok(arg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_resolve_raw_string() {
        let result = resolve_input("3 + 4, 10 * 2");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "3 + 4, 10 * 2");
    }

    #[test]
    fn test_resolve_file_contents() {
        let path = "/tmp/test_resolve_input_file.txt";
        fs::write(path, "1 + 2, 3 * 4\n").expect("failed to write temp file");

        let result = resolve_input(path);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("1 + 2"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_resolve_nonexistent_path_treated_as_raw() {
        // A path that doesn't exist should be treated as a raw equation string
        let result = resolve_input("1 + 2");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1 + 2");
    }

    #[test]
    fn test_resolve_nonexistent_file_path_treated_as_raw() {
        // A path that looks like a file path but doesn't exist is treated as raw input
        let nonexistent = "/tmp/nonexistent_file_xyz.txt";
        // Ensure it really doesn't exist
        let _ = fs::remove_file(nonexistent);
        let result = resolve_input(nonexistent);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), nonexistent);
    }

    #[test]
    fn test_resolve_valid_file_path_returns_contents() {
        // A valid file path should return the file's contents
        let path = "/tmp/test_resolve_valid_file_path.txt";
        let contents = "5 + 6, 7 * 8";
        fs::write(path, contents).expect("failed to write temp file");

        let result = resolve_input(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), contents);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_resolve_directory_returns_error() {
        // A directory path should return FileReadError
        let result = resolve_input(".");
        assert!(matches!(result, Err(InputError::FileReadError(_))));
    }
}
