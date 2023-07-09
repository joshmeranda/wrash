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

## Installation

### >= v0.3.0

`go install github.com/joshmeranda/wrash@<version>`

### < v0.3.0

Wrash available on the [Arch User Repository](https://aur.archlinux.org/packages/wrash-git) (AUR).

For version `v0.2.0` we ported this project from [Rust](https://doc.rust-lang.org/) to [Golang](https://go.dev/), so the manual build / install process is differnt for those versions.

#### Rust

```
cargo install --path .
```

#### Golang

```
make wrash
ln -s $(realpath bin/wrash) /usr/bin
```

### < v0.2.0
Clone, build, and install with [Cargo](https://doc.rust-lang.org/cargo/).
