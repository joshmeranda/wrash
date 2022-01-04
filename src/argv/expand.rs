use std::env;
use crate::argv::error::ArgumentError;

/// Expand all found parameter expansions, bot in and outside of double quotes.
fn expand_vars(arg: &str) -> Result<String, ArgumentError>{
    let mut expanded = String::new();
    let mut chars = arg.chars().enumerate().peekable();
    let mut last = 0;

    while let Some((i, c)) = chars.next() {
        match c {
            '$' => {
                if i != 0 {
                    expanded.push_str(&arg[last..i]);
                }

                let (name, next) = match chars.peek() {
                    None => return Err(ArgumentError::UnterminatedSequence('$')),
                    Some((_, '{')) => {
                        if let Some(n) = arg[i + 1..].find('}') {
                            (&arg[i + 2..i + n + 1], i + n + 2)
                        } else {
                            return Err(ArgumentError::UnterminatedSequence('{'))
                        }
                    },
                    Some((_, _)) => {
                        if let Some(n) = arg[i + 1..].find(|c: char| !c.is_alphanumeric() && c != '_') {
                            (&arg[i + 1..i + n + 1], i + n + 1)
                        } else {
                            (&arg[i + 1..], arg.len())
                        }
                    },
                };
                let value = env::var(name).unwrap_or_else(|_| String::new());

                expanded.push_str(value.as_str());

                last = next;
            },
            '\'' => { chars.find(|(_, c)| *c == '\''); },
            _ => { /* do nothing */ },
        }
    }

    if last < arg.len() {
        expanded.push_str(&arg[last..]);
    }

    Ok(expanded)
}

/// Remove all non-escaped strings.
fn expand_quotes(arg: &str) -> Result<String, ArgumentError> {
    let mut expanded = String::new();
    let mut chars = arg.chars();

    let mut is_single_quote = false;
    let mut is_double_quote = false;

    while let Some(c) = chars.next() {
        match c {
            '\'' => if ! is_double_quote {
                is_single_quote = ! is_single_quote;
            } else {
                expanded.push(c);
            },
            '"' => if ! is_single_quote {
                is_double_quote = ! is_double_quote;
            } else {
                expanded.push(c);
            },
            '*' | '?' => {
                expanded.push('\\');
                expanded.push(c);
            },
            '\\' => if let Some(c) = chars.next() {
                if matches!(c, '"' | '\'' | ' ') {
                    expanded.push(c);
                } else {
                    return Err(ArgumentError::InvalidEscape(c))
                }
            } else {
                return Err(ArgumentError::UnexpectedEndOfLine)
            },
            _ => expanded.push(c)
        }
    }

    Ok(expanded)
}

fn expand_globs(arg: &str) -> Vec<String> {
    todo!()
}

fn expand(arg: &str) -> Vec<String> {
    todo!()
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    fn get_resource_path(components: &[&str]) -> PathBuf {
        vec!["tests", "resources"]
            .iter()
            .chain(components.iter())
            .collect()
    }

    mod test_vars {
        use std::env;
        use crate::argv::expand;

        #[test]
        fn test_no_vars() {
            let expected = Ok("abcd".to_string());
            let actual = expand::expand_vars("abcd");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_var_with_curly() {
            env::set_var("A", "a");

            let expected = Ok("a".to_string());
            let actual = expand::expand_vars("${A}");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_var_no_curly() {
            env::set_var("A", "a");

            let expected = Ok("a".to_string());
            let actual = expand::expand_vars("$A");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_leading_var_with_curly() {
            env::set_var("A", "a");

            let expected = Ok("abc".to_string());
            let actual = expand::expand_vars("${A}bc");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_multiple_vars() {
            env::set_var("A", "a");
            env::set_var("C", "c");

            let expected = Ok("a b c".to_string());
            let actual = expand::expand_vars("$A b ${C}");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_expansion_in_double_quotes() {
            env::set_var("A", "a");

            let expected = Ok("\"a\"".to_string());
            let actual = expand::expand_vars("\"${A}\"");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_expansion_in_single_quotes() {
            env::set_var("A", "a");

            let expected = Ok("'${A}'".to_string());
            let actual = expand::expand_vars("'${A}'");

            assert_eq!(expected, actual);
        }
    }

    mod test_quotes {
        use std::env;
        use crate::argv::expand;

        #[test]
        fn test_expand_single() {
            let expected = Ok("abc".to_string());
            let actual = expand::expand_quotes("a'b'c");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_expand_double() {
            let expected = Ok("abc".to_string());
            let actual = expand::expand_quotes("a\"b\"c");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_single_quote_inside_double() {
            let expected = Ok("a'bc".to_string());
            let actual = expand::expand_quotes("a\"'\"bc");

            assert_eq!(expected, actual);
        }
        #[test]
        fn test_single_escaped_quote_inside_double() {
            let expected = Ok("a\"bc".to_string());
            let actual = expand::expand_quotes("a\"\\\"\"bc");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_expand_escaped_quote() {
            let expected = Ok("a'bc".to_string());
            let actual = expand::expand_quotes("a\\'bc");

            assert_eq!(expected, actual);
        }
    }

    mod test_globs {
        use crate::argv::{expand, split};
        use std::env;
        use std::path::PathBuf;
        use crate::argv::expand::test::get_resource_path;

        #[test]
        fn test_existing_glob() -> Result<(), Box<dyn std::error::Error>> {
            let arg = "a*file";

            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory"]);

            let expected = vec![
                "a_file".to_string(),
                "another_file".to_string(),
            ];

            env::set_current_dir(new_cwd)?;
            let actual = expand::expand(arg);
            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            Ok(())
        }

        #[test]
        fn test_no_existing_glob() -> Result<(), Box<dyn std::error::Error>> {
            let arg = "b*file";

            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory"]);

            let expected: Vec<String> = vec![];

            env::set_current_dir(new_cwd)?;
            let actual = expand::expand(arg);
            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            Ok(())
        }
    }

    mod test_expand {
        use std::env;
        use crate::argv::expand;
        use crate::argv::expand::test::get_resource_path;

        #[ignore]
        #[test]
        fn test_glob_with_string() -> Result<(), Box<dyn std::error::Error>> {
            let arg = "a*file'abc'";

            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory"]);

            let expected = vec![
                "a_file'abc'".to_string(),
                "another_file'abc'".to_string(),
            ];

            env::set_current_dir(new_cwd)?;
            let actual = expand::expand(arg);
            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            Ok(())
        }

        #[ignore]
        #[test]
        fn test_expand_single_quoted_glob() -> Result<(), Box<dyn std::error::Error>> {
            let arg = "'a*file'";

            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory"]);

            let expected = vec!["a*file".to_string()];

            env::set_current_dir(new_cwd)?;
            let actual = expand::expand(arg);
            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            Ok(())
        }

        #[ignore]
        #[test]
        fn test_expand_with_double_quoted_glob() -> Result<(), Box<dyn std::error::Error>> {
            let arg = "\"a*file\"";

            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory"]);

            let expected = vec!["a*file".to_string()];

            env::set_current_dir(new_cwd)?;
            let actual = expand::expand(arg);
            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            Ok(())
        }

        #[ignore]
        #[test]
        fn test_expand_glob_no_quotes() -> Result<(), Box<dyn std::error::Error>> {
            let arg = "a*file";

            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory"]);

            let expected = vec![String::new()];

            env::set_current_dir(new_cwd)?;
            let actual = expand::expand(arg);
            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            Ok(())
        }

        #[test]
        fn test_expand_mid_word_quotes() -> Result<(), Box<dyn std::error::Error>> {
            let arg = "a'_'file";

            let expected = vec!["a_file".to_string()];
            let actual = expand::expand_globs(arg);

            assert_eq!(expected, actual);

            Ok(())
        }
    }
}