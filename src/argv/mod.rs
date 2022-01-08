pub mod error;
pub mod expand;

/// Provides much of the same functionality as `Iterator::find` but also
/// provides the previous value if it exists. Use `previous` to test the
/// previous value, and `current` to test the current value (including the the
/// first value). `previous` takes an optional to allow you to specify whether
/// a `true` return from `current` on the first element will stop any further
/// iteration or not.
fn find_with_previous<I, F, G>(iterator: &mut I, previous: F, current: G) -> Option<I::Item>
    where
        I: Iterator,
        F: Fn(Option<&I::Item>) -> bool,
        G: Fn(&I::Item) -> bool,
{
    match iterator.next() {
        None => None,
        Some(current_item) => {
            if previous(None) && current(&current_item) {
                Some(current_item)
            } else {
                let mut previous_item = current_item;

                // while let Some(current_item) = iterator.next() {
                for current_item in iterator {
                    if previous(Some(&previous_item)) && current(&current_item) {
                        return Some(current_item);
                    }

                    previous_item = current_item;
                }

                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    mod find_with_previous {
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