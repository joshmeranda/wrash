# Wrash
Wrash is an interactive shell wrapper for you favorite commands!

Wrash will save you some precious key-hits when you have to run multiple subcommands to do something. Instead of running 

```sh
>>> git add src/source_code.ext
>>> git status
>>> git commit -m 'fix some bug'
>>> git push
```

You can run:

```sh
>>> wrash git
>>> add src/source_code.ext
>>> status
>>> commit -m 'fix some bug'
>>> push
```

## Instalattion
Wrash v0.1 is now available on the [Arch User Repository](https://aur.archlinux.org/packages/wrash-git) (AUR).

Or clone, build, and install with [Cargo](https://doc.rust-lang.org/cargo/).

## Testing
Most tests can be run with the typical `cargo test`, but some require that we mutate the current working directory,
and therefore might cause false-negatives in other tests that access files with relatives paths. To mitigate this, all
of these tests are `#[ignored]` in most situations; however, they can be run with
`cargo test -- --test-threads 1 --ignored` whenever necessary or run everything at once with
`cargo test -- --test-threads 1 --include-ignored`.

Many of those same tests use relative paths when setting the working directory, meaning that if tests are not run from
the project root, they are likely to fail.
