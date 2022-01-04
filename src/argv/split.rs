use crate::argv::error::ArgumentError;
use crate::argv;

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

    /// Skip all whitespace in `source` while incrementing `offset`.
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
        if self.has_err || self.offset >= self.source.len() {
            return None
        }

        self.skip_whitespace();
        let mut chars = self.source.chars();

        if let Some(c) = chars.nth(self.offset) {
            // check the first character for the first step
            match c {
                // todo: combine both quoting branches toigether usine `c` instead o f each quote chracer individually
                '\'' =>
                    if let Some(n) = self.source[self.offset + 1..].find(|c| c == '\'') {
                        let r = Some(Ok(&self.source[self.offset..n + 1]));
                        self.offset = n + 1;

                        r
                    } else {
                        self.has_err = true;
                        Some(Err(ArgumentError::UnterminatedSequence('\'')))
                    },
                '"' =>
                    if let Some(n) = self.source[self.offset + 1..].find(|c| c == '"') {
                        let r = Some(Ok(&self.source[self.offset..n + 1]));
                        self.offset = n + 1;

                        r
                    } else {
                        self.has_err = true;
                        Some(Err(ArgumentError::UnterminatedSequence('"')))
                    },
                _ => if let Some((n, _)) = argv::find_with_previous(&mut chars.enumerate(),
                                                              |o| if let Some((_, c)) = o { *c != '\\' } else { true },
                                                              |(_, c)| c.is_whitespace()
                ) {
                    let r = Some(Ok(&self.source[self.offset..self.offset + n + 1]));
                    self.offset += n + 1;

                    r
                } else {
                    let r = Some(Ok(&self.source[self.offset..]));
                    self.offset = self.source.len();

                    r
                }
            }
        } else {
            None
        }
    }
}

/// Split a full list of command line arguments into their separate args.
///
/// todo: ideally we could return an `impl iterator` to hide our internal `Split` struct.
fn split(source: &str) -> Split {
    Split::new(source)
}

#[cfg(test)]
mod split {
    use crate::argv;
    use crate::argv::error::ArgumentError;
    use crate::argv::split;

    #[test]
    fn test_empty() -> Result<(), Box<dyn std::error::Error>> {
        let source = "";
        let mut actual = split::split(source);

        assert_eq!(None, actual.next());

        Ok(())
    }

    #[test]
    fn test_command_only() -> Result<(), Box<dyn std::error::Error>> {
        let source = "cmd";
        let mut actual = split::split(source);

        assert_eq!(Some(Ok("cmd")), actual.next());
        assert_eq!(None, actual.next());

        Ok(())
    }

    #[test]
    fn test_command_with_args() -> Result<(), Box<dyn std::error::Error>> {
        let source = "cmd a b c";
        let mut actual = split::split(source);

        assert_eq!(Some(Ok("cmd")), actual.next());
        assert_eq!(Some(Ok("a")), actual.next());
        assert_eq!(Some(Ok("b")), actual.next());
        assert_eq!(Some(Ok("c")), actual.next());
        assert_eq!(None, actual.next());

        Ok(())
    }

    #[test]
    fn test_unterminated_single_string() -> Result<(), Box<dyn std::error::Error>> {
        let source = "cmd a 'b c";
        let mut actual = split::split(source);

        assert_eq!(Some(Ok("cmd")), actual.next());
        assert_eq!(Some(Ok("a")), actual.next());
        assert_eq!(
            Some(Err(ArgumentError::UnterminatedSequence('\''))),
            actual.next()
        );
        assert_eq!(None, actual.next());

        Ok(())
    }

    #[test]
    fn test_unterminated_double_string() -> Result<(), Box<dyn std::error::Error>> {
        let source = "cmd a \"b c";
        let mut actual = split::split(source);

        assert_eq!(Some(Ok("cmd")), actual.next());
        assert_eq!(Some(Ok("a")), actual.next());
        assert_eq!(
            Some(Err(ArgumentError::UnterminatedSequence('\"'))),
            actual.next()
        );
        assert_eq!(None, actual.next());

        Ok(())
    }

    #[test]
    fn test_unterminated_mixed() -> Result<(), Box<dyn std::error::Error>> {
        let source = "cmd a b\\ 'c'";
        let mut actual = split::split(source);

        assert_eq!(Some(Ok("cmd")), actual.next());
        assert_eq!(Some(Ok("a")), actual.next());
        assert_eq!(Some(Ok("b\\ 'c'")), actual.next());
        assert_eq!(None, actual.next());

        Ok(())
    }
}