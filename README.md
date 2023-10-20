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

### >= v0.4.0

The simplest method is to run `make install`. This leverages `go install` to do the installtion for us so make sure your path is configured properlly to point to your `$GOROOT/bin` dir.

Otherwise you can follow the same instructions for `>= v0.3.0`

### >= v0.3.0

The simplest way is to install vi `go` with the command below:

```
go install github.com/joshmeranda/wrash@<version>
```

NOTE: because the version is injected at build tikme if you want to check what version you have installed via `wrash --version` you will need to pass the `ldflags "-X main.Version=<version>` flag as well (replacing `<version>` with the desired version). Thi is fixed in `v0.4.0`

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
