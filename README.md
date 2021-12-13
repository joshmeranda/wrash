# Wrash


## Testing
Most tests can be run with the typical `cargo test`, but some require that we mutate the current working directory,
and therefore might cause false-negatives in other tests that access files with relatives paths. To mitigate this, all
of these tests are `#[ignored]` in most situations; however, they can be run with
`cargo test -- --test-threads 1 --ignored` whenever necessary or run everything at once with
`cargo test -- --test-threads 1 --include-ignored`.

Many of those same tests use relative paths when setting the working directory, meaning that if tests are not run from
the project root, they are likely to fail.