use std::path::Component::ParentDir;
use std::path::Iter;
use clap::Arg;
use crate::argv::error::ArgumentError;

mod error;

/// Provides much of the same functionality as `Iterator::find` but also
/// provides the previous value if it exists. Use `g` to test the previous
/// value, and `g` to test the current value (including the the first value).
/// `f` takes an optional to allow you to specify whether a `true` return from
/// `g` on the first element will stop any further iteration or not.
fn find_with_previous<I, F, G>(iterator: &mut I, mut f: F, mut g: G) -> Option<I::Item>
    where
        I: Iterator,
        F: FnMut(Option<&I::Item>) -> bool,
        G: FnMut(&I::Item) -> bool,
{
    let mut current: Option<I::Item> = iterator.next();

    match current {
        None => None,
        Some(current) => {
            if f(None) && g(&current) {
                Some(current)
            } else {
                let mut previous = current;

                while let Some(current) = iterator.next() {
                    if f(Some(&previous)) && g(&current) {
                        return Some(current);
                    }

                    previous = current;
                }

                None
            }
        }
    }
}

/// An argument splitter which preserves any delimiter and quotes if finds
/// `Split` internally tracks any errors it finds, and after the first error
/// is found, any calls to `next` will return `None.
struct Split<'a> {
    source: &'a str,

    offset: usize,

    has_err: bool,
}

impl <'a> Split<'a> {
    fn new(source: &'a str) -> Split {
        Split {
            source,
            offset: 0,
            has_err: false,
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.source.chars().nth(self.offset) {
            if ! c.is_whitespace() {
                break
            }

            self.offset += 1;
        }
    }
}

impl <'a> Iterator for Split<'a> {
    type Item = Result<&'a str, ArgumentError>;

    fn next(&mut self) -> Option<Self::Item> {
        // if we have already encountered a parsing error we don't need to continue
        if self.has_err {
            return None
        }

        self.skip_whitespace();
        let mut chars = self.source.chars();

        if let Some(c) = chars.nth(self.offset) {
            // check the first character for the first step
            match c {
                '\'' =>
                    if let Some(n) = self.source.find(|c| c == '\'') {
                        self.offset = n;
                        Some(Ok(&self.source[self.offset..n + 1]))
                    } else {
                        Some(Err(ArgumentError::UnterminatedSequence('\'')))
                    },
                '"' =>
                    if let Some(n) = self.source.find(|c| c == '"') {
                        self.offset = n;
                        Some(Ok(&self.source[self.offset..n + 1]))
                    } else {
                        Some(Err(ArgumentError::UnterminatedSequence('"')))
                    },
                _ => if let Some((n, _)) = find_with_previous(&mut chars.enumerate(),
                    |o| if let Some((_, c)) = o { *c != '\\' } else { true },
                    |(_, c)| c.is_whitespace()
                ) {
                    self.offset = n;
                    Some(Ok(&self.source[self.offset..n + 1]))
                } else {
                    Some(Ok(&self.source[self.offset..]))
                }
            }
        } else {
            None
        }
    }
}

/// Split a full list of command line arguments into their separate args.
///
/// todo: ideally we could return an `impl iterator` to hid out internal `Split` struct.
fn split(source: &str) -> Split {
    Split::new(source)
}

#[cfg(test)]
mod test {
    mod split {
        use crate::argv;
        use crate::argv::error::ArgumentError;

        #[test]
        fn test_empty() -> Result<(), Box<dyn std::error::Error>> {
            let source = "";
            let mut actual = argv::split(source);

            assert_eq!(None, actual.next());

            Ok(())
        }

        #[test]
        fn test_command_only() -> Result<(), Box<dyn std::error::Error>> {
            let source = "cmd";
            let mut actual = argv::split(source);

            assert_eq!(Some(Ok("cmd")), actual.next());
            assert_eq!(None, actual.next());

            Ok(())
        }

        #[test]
        fn test_command_with_args() -> Result<(), Box<dyn std::error::Error>> {
            let source = "cmd a b c";
            let mut actual = argv::split(source);

            assert_eq!(Some(Ok("cmd")), actual.next());
            assert_eq!(Some(Ok("a")), actual.next());
            assert_eq!(Some(Ok("b")), actual.next());
            assert_eq!(Some(Ok("c")), actual.next());
            assert_eq!(None, actual.next());

            Ok(())
        }

        #[test]
        fn test_unterminated_string() -> Result<(), Box<dyn std::error::Error>> {
            let source = "cmd a 'b c";
            let mut actual = argv::split(source);

            assert_eq!(Some(Ok("cmd")), actual.next());
            assert_eq!(Some(Ok("a")), actual.next());
            assert_eq!(
                Some(Err(ArgumentError::UnterminatedSequence('\''))),
                actual.next()
            );
            assert_eq!(None, actual.next());

            Ok(())
        }
    }

    mod find_with_previous {
        use crate::argv::find_with_previous;
        use std::ops::Range;
        use crate::argv;

        #[test]
        fn test_empty() {
            let mut iter = 0..0;

            let expected = None;
            let actual = argv::find_with_previous(
                &mut iter,
                |n| if let Some(n) = n { *n == 0 } else { true },
                |n| *n == 1,
            );

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_no_previous_allow_pass() {
            let mut iter = 0..1;

            let expected = Some(0);
            let actual = argv::find_with_previous(
                &mut iter,
                |n| if let Some(n) = n { *n == 0 } else { true },
                |n| *n == 0,
            );

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_no_previous_block_pass() {
            let mut iter = 0..1;

            let expected = None;
            let actual = argv::find_with_previous(
                &mut iter,
                |n| if let Some(n) = n { *n == 0 } else { false },
                |n| *n == 1,
            );

            assert_eq!(expected, actual);
        }

        #[test]
        fn test_with_previous() {
            let mut iter = 0..10;

            let expected = Some(9);
            let actual = argv::find_with_previous(
                &mut iter,
                |n| if let Some(n) = n { *n == 8 } else { false },
                |n| *n == 9,
            );

            assert_eq!(expected, actual);
        }
    }
}