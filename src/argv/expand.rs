/// Expands all parameter expansions
fn expand_vars(arg: &str) -> String {
    todo!()
}

fn expand_quotes(arg: &str) -> String {
    todo!()
}

fn expand_globs(arg: &str) -> Vec<String> {
    todo!()
}

fn expand(arg: &str) -> Vec<String> {
    todo!()
}

#[cfg(test)]dd
mod test {
    mod test_vars {
        use std::env;
        use crate::argv::expand;

        #[test]
        fn test_no_vars() {
            let expected = "abcd".to_string();
            let expected = expand::expand_vars("abcd");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_var_with_curly() {
            env::set_var("A", "a");

            let expected = "a".to_string();
            let actual = expand::expand_vars("${A}");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_var_no_curly() {
            env::set_var("A", "a");

            let expected = "a".to_string();
            let actual = expand::expand_vars("$A");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_leading_var_with_curly() {
            env::set_var("A", "a");

            let expected = "abc".to_string();
            let actual = expand::expand_vars("${A}bc");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_multiple_vars() {
            env::set_var("A", "a");
            env::set_var("C", "c");

            let expected = "a b c".to_string();
            let actual = expand::expand_vars("$A b ${C}");

            assert_eq!(expected, actual);
        }
    }

    mod test_quotes {
        use std::env;
        use crate::argv::expand;

        #[test]
        fn test_expand_single() {
            let expected = "abc".to_string();
            let actual = expand::expand_quotes("a'b'c");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_expand_double() {
            let expected = "abc".to_string();
            let actual = expand::expand_quotes("a\"b\"c");

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_expand_double_with_var() {
            env::set_var("B", "b");

            let expected = "abc".to_string();
            let actual = expand::expand_quotes("a\"B\"c");

            assert_eq!(expected, actual);
        }
    }

    mod test_globs {
        use crate::argv::expand::Expander;
        use crate::argv::{expand, split};
        use std::env;
        use std::path::PathBuf;

        fn get_resource_path(components: &[&str]) -> PathBuf {
            vec!["tests", "resources"]
                .iter()
                .chain(components.iter())
                .collect()
        }

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

            let expected = vec![];

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