use std::env;

use crate::argv;
use crate::argv::error::ArgumentError;

/// Replace all non-quoted  and non-escaped tildes with the user's home
/// directory.The `provider` should be a simple method that returns the user's
/// home directory as a string, or None if the home directory could not be
/// determined. If the user's home directory could not be determined, the tilde
/// is not expanded and left as-is.
fn expand_tilde<F>(source: &str, provider: F) -> Result<String, ArgumentError>
where
    F: FnOnce() -> Option<String>,
{
    if !source.contains('~') {
        Ok(source.to_string())
    } else {
        let mut expanded = String::new();
        let mut chars = source.chars().enumerate();

        let home = match provider() {
            Some(home) => home,
            None => "~".to_string(),
        };

        while let Some((start, c)) = chars.next() {
            match c {
                '\'' | '"' => {
                    if let Some((end, _)) = argv::find_with_previous(
                        &mut chars,
                        |o| {
                            if let Some((_, c)) = o {
                                *c != '\\'
                            } else {
                                true
                            }
                        },
                        |(_, current)| *current == c,
                    ) {
                        expanded.push_str(&source[start..end + 1]);
                    } else {
                        return Err(ArgumentError::UnterminatedSequence(c));
                    }
                }
                '\\' => match chars.next() {
                    Some((_, '~')) => expanded.push('~'),
                    Some((_, c)) => {
                        expanded.push('\\');
                        expanded.push(c);
                    }
                    None => return Err(ArgumentError::UnterminatedSequence('\\')),
                },
                '~' => expanded.push_str(home.as_str()),
                _ => expanded.push(c),
            }
        }

        Ok(expanded)
    }
}

/// Expand all found parameter expansions, bot in and outside of double quotes.
fn expand_vars(source: &str) -> Result<String, ArgumentError> {
    let mut expanded = String::new();
    let mut chars = source.chars().enumerate().peekable();
    let mut last = 0;

    while let Some((i, c)) = chars.next() {
        match c {
            '$' => {
                if i != 0 {
                    expanded.push_str(&source[last..i]);
                }

                let (name, next) = match chars.peek() {
                    None => return Err(ArgumentError::UnterminatedSequence('$')),
                    Some((_, '{')) => {
                        if let Some(n) = source[i + 1..].find('}') {
                            (&source[i + 2..i + n + 1], i + n + 2)
                        } else {
                            return Err(ArgumentError::UnterminatedSequence('{'));
                        }
                    }
                    Some((_, _)) => {
                        if let Some(n) =
                            source[i + 1..].find(|c: char| !c.is_alphanumeric() && c != '_')
                        {
                            (&source[i + 1..i + n + 1], i + n + 1)
                        } else {
                            (&source[i + 1..], source.len())
                        }
                    }
                };
                let value = env::var(name).unwrap_or_else(|_| String::new());

                expanded.push_str(value.as_str());

                last = next;
            }
            '\'' => {
                chars.find(|(_, c)| *c == '\'');
            }
            _ => { /* do nothing */ }
        }
    }

    if last < source.len() {
        expanded.push_str(&source[last..]);
    }

    Ok(expanded)
}

/// Split a line into its individual words.
fn split_words(source: &str) -> Vec<&str> {
    let mut words = vec![];
    let mut chars = source.chars().enumerate();
    let mut last = 0;

    while let Some((i, c)) = chars.next() {
        match c {
            ' ' | '\t' => {
                if last == i {
                    last += 1;
                } else {
                    words.push(&source[last..i]);
                    last = i + 1;
                }
            }
            '\'' | '"' => {
                argv::find_with_previous(
                    &mut chars,
                    |o| {
                        if let Some((_, c)) = o {
                            *c != '\\'
                        } else {
                            true
                        }
                    },
                    |(_, current)| *current == c,
                );
            }
            '\\' => {
                chars.next();
            }
            _ => {}
        }
    }

    if last < source.len() {
        words.push(&source[last..])
    }

    words
}

fn is_pattern(s: &str) -> bool {
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        match c {
            '?' | '*' | '[' => return true,
            '\\' => {
                chars.next();
            }
            _ => {}
        }
    }

    false
}

/// Check each arg in `argv` and if it contains any of `*`, `?`, or `[` it is
/// regarded as a glob and will be expanded. If there are matches to the
/// pattern on the filesystem, the raw pattern string will be returned.
fn expand_filenames(argv: Vec<&str>) -> Vec<String> {
    let mut expanded = vec![];

    for arg in argv {
        if !is_pattern(arg) {
            expanded.push(arg.to_string());
        } else if let Ok(paths) = glob::glob(arg) {
            let mut found: Vec<String> = paths
                .filter_map(|r| match r {
                    Ok(p) => Some(p.to_string_lossy().to_string()),
                    Err(_) => None,
                })
                .collect();

            if found.is_empty() {
                expanded.push(arg.to_string());
            } else {
                expanded.append(&mut found);
            }
        }
    }

    expanded
}

/// Remove all non-escaped strings.
fn expand_quotes(argv: Vec<String>) -> Result<Vec<String>, ArgumentError> {
    let mut words = vec![];

    for word in argv {
        let mut expanded = String::new();
        let mut chars = word.chars();

        let mut is_single_quote = false;
        let mut is_double_quote = false;

        while let Some(c) = chars.next() {
            match c {
                '\'' => {
                    if !is_double_quote {
                        is_single_quote = !is_single_quote;
                    } else {
                        expanded.push(c);
                    }
                }
                '"' => {
                    if !is_single_quote {
                        is_double_quote = !is_double_quote;
                    } else {
                        expanded.push(c);
                    }
                }
                '\\' => {
                    if let Some(c) = chars.next() {
                        if matches!(c, '"' | '\'' | ' ' | '~') {
                            expanded.push(c);
                        } else {
                            return Err(ArgumentError::InvalidEscape(c));
                        }
                    } else {
                        return Err(ArgumentError::UnexpectedEndOfLine);
                    }
                }
                _ => expanded.push(c),
            }
        }

        if is_single_quote {
            return Err(ArgumentError::UnterminatedSequence('\''))
        } else if is_double_quote {
            return Err(ArgumentError::UnterminatedSequence('\''))
        } else {
            words.push(expanded);
        }
    }

    Ok(words)
}

/// Expands a line of input in a similar order to Bash as described in the
/// [Shell Expansions](https://www.gnu.org/software/bash/manual/html_node/Shell-Expansions.html)
/// section of the documentation:
///
/// > The order of expansions is: brace expansion; tilde expansion, parameter
/// and variable expansion, arithmetic expansion, and command substitution
/// (done in a left-to-right fashion); word splitting; and filename expansion.
/// >
/// > On systems that can support it, there is an additional expansion available:
/// process substitution. This is performed at the same time as tilde,
/// parameter, variable, and arithmetic expansion and command substitution.
/// >
/// > After these expansions are performed, quote characters present in the
/// original word are removed unless they have been quoted themselves (quote
/// removal).
///
/// Since wrash does not support [Brace Expansion](https://www.gnu.org/software/bash/manual/html_node/Brace-Expansion.html),
/// [Command Substitution](https://www.gnu.org/software/bash/manual/html_node/Command-Substitution.html),
/// [Arithmetic Expansion](https://www.gnu.org/software/bash/manual/html_node/Arithmetic-Expansion.html),
/// or [Process Substitution](https://www.gnu.org/software/bash/manual/html_node/Process-Substitution.html)
/// those steps are ignored.
///
/// todo: validate sequences and escapes before expanding
/// todo: tilde (~) expansion
/// todo: variable / parameter expansion
/// todo: word splitting
/// todo: filename expansion
pub fn expand(source: &str) -> Result<Vec<String>, ArgumentError> {
    let tilde = expand_tilde(source, || {
        dirs::home_dir().map(|p| p.to_string_lossy().to_string())
    })?;

    let variable = expand_vars(tilde.as_str())?;

    let words = split_words(variable.as_str());

    let filenames = expand_filenames(words);

    let quotes = expand_quotes(filenames)?;

    Ok(quotes)
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

    mod test_tilde {
        use crate::argv::expand;

        #[test]
        fn test_simple() {
            let expected = Ok("HOME".to_string());
            let actual = expand::expand_tilde("~", || Some("HOME".to_string()));

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_simple_with_child() {
            let expected = Ok("HOME/a".to_string());
            let actual = expand::expand_tilde("~/a", || Some("HOME".to_string()));

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_quoted() {
            let expected = Ok("'~'/a".to_string());
            let actual = expand::expand_tilde("'~'/a", || Some("HOME".to_string()));

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_escaped() {
            let expected = Ok("~/a".to_string());
            let actual = expand::expand_tilde("\\~/a", || Some("HOME".to_string()));

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_no_home() {
            let expected = Ok("~".to_string());
            let actual = expand::expand_tilde("~", || None);

            assert_eq!(expected, actual)
        }
    }

    mod test_vars {
        use crate::argv::expand;
        use std::env;

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
        fn test_var_no_curly_with_trailing_bang() {
            env::set_var("A", "a");

            let expected = Ok("a!".to_string());
            let actual = expand::expand_vars("$A!");

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

    mod test_word_split {
        use crate::argv::expand;

        #[test]
        fn test_one_word() {
            let expected = vec!["a"];
            let actual = expand::split_words("a");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_with_space() {
            let expected = vec!["a", "b"];
            let actual = expand::split_words("a b");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_with_tab() {
            let expected = vec!["a", "b"];
            let actual = expand::split_words("a\tb");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_with_trailing_delimiter() {
            let expected = vec!["a", "b"];
            let actual = expand::split_words("a b ");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_with_extra_delimiter() {
            let expected = vec!["a", "b"];
            let actual = expand::split_words("a  b");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_ignore_in_quotes() {
            let expected = vec!["'a b'"];
            let actual = expand::split_words("'a b'");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_ignore_escaped() {
            let expected = vec!["a\\ b"];
            let actual = expand::split_words("a\\ b");

            assert_eq!(expected, actual);
        }
    }

    mod test_filename {
        use crate::argv::expand;
        use crate::argv::expand::test::get_resource_path;
        use std::env;

        #[ignore]
        #[test]
        fn test_existing_glob() -> Result<(), Box<dyn std::error::Error>> {
            let args = vec!["a*file"];

            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory"]);

            let expected = vec!["a_file".to_string(), "another_file".to_string()];

            env::set_current_dir(new_cwd)?;
            let actual = expand::expand_filenames(args);
            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            Ok(())
        }

        #[ignore]
        #[test]
        fn test_escaped_glob_no_existing() -> Result<(), Box<dyn std::error::Error>> {
            let args = vec!["a\\*file"];

            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory"]);

            let expected = vec!["a\\*file".to_string()];

            env::set_current_dir(new_cwd)?;
            let actual = expand::expand_filenames(args);
            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            Ok(())
        }

        #[ignore]
        #[test]
        fn test_no_existing_glob() -> Result<(), Box<dyn std::error::Error>> {
            let args = vec!["b*file"];

            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory"]);

            let expected = vec!["b*file".to_string()];

            env::set_current_dir(new_cwd)?;
            let actual = expand::expand_filenames(args);
            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            Ok(())
        }
    }

    mod test_quotes {
        use crate::argv::error::ArgumentError;
        use crate::argv::expand;

        #[test]
        fn test_expand_single() {
            let expected = Ok(vec!["abc".to_string()]);
            let actual = expand::expand_quotes(vec!["a'b'c".to_string()]);

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_expand_double() {
            let expected = Ok(vec!["abc".to_string()]);
            let actual = expand::expand_quotes(vec!["a\"b\"c".to_string()]);

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_single_quote_inside_double() {
            let expected = Ok(vec!["a'bc".to_string()]);
            let actual = expand::expand_quotes(vec!["a\"'\"bc".to_string()]);

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_single_escaped_quote_inside_double() {
            let expected = Ok(vec!["a\"bc".to_string()]);
            let actual = expand::expand_quotes(vec!["a\"\\\"\"bc".to_string()]);

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_expand_escaped_quote() {
            let expected = Ok(vec!["a'bc".to_string()]);
            let actual = expand::expand_quotes(vec!["a\\'bc".to_string()]);

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_quoted_glob_character() {
            let expected = Ok(vec!["a*b".to_string()]);
            let actual = expand::expand_quotes(vec!["a'*'b".to_string()]);

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_unterminated_quote() {
            let expected = Err(ArgumentError::UnterminatedSequence('\''));
            let actual = expand::expand_quotes(vec!["cmd a 'b c".to_string()]);

            assert_eq!(expected, actual);
        }
    }

    mod test_expand {
        use crate::argv::expand;
        use crate::argv::expand::test::get_resource_path;
        use std::env;
        use crate::argv::error::ArgumentError;

        #[ignore]
        #[test]
        fn test_glob_with_string() -> Result<(), Box<dyn std::error::Error>> {
            let source = "a*file'abc'";

            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory"]);

            let expected = vec!["a*fileabc".to_string()];

            env::set_current_dir(new_cwd)?;
            let actual = expand::expand(source)?;
            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            Ok(())
        }

        #[ignore]
        #[test]
        fn test_expand_single_quoted_glob() -> Result<(), Box<dyn std::error::Error>> {
            let source = "'a*file'";

            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory"]);

            let expected = vec!["a*file".to_string()];

            env::set_current_dir(new_cwd)?;
            let actual = expand::expand(source)?;
            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            Ok(())
        }

        #[ignore]
        #[test]
        fn test_expand_with_double_quoted_glob() -> Result<(), Box<dyn std::error::Error>> {
            let source = "\"a*file\"";

            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory"]);

            let expected = vec!["a*file".to_string()];

            env::set_current_dir(new_cwd)?;
            let actual = expand::expand(source)?;
            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            Ok(())
        }

        #[ignore]
        #[test]
        fn test_expand_glob_no_quotes() -> Result<(), Box<dyn std::error::Error>> {
            let source = "a*file";

            let old_cwd = env::current_dir()?;
            let new_cwd = get_resource_path(&["a_directory"]);

            let expected = vec!["a_file".to_string(), "another_file".to_string()];

            env::set_current_dir(new_cwd)?;
            let actual = expand::expand(source)?;
            env::set_current_dir(old_cwd)?;

            assert_eq!(expected, actual);

            Ok(())
        }

        #[test]
        fn test_expand_mid_word_quotes() -> Result<(), Box<dyn std::error::Error>> {
            let source = "a'_'file";

            let expected = vec!["a_file".to_string()];
            let actual = expand::expand(source)?;

            assert_eq!(expected, actual);

            Ok(())
        }
    }
}
