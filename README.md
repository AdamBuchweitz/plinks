# plinks

`plinks` is a project-local link manager for Rust projects and other repos. It stores links in a checked-in `project-links.toml` file and provides both shell commands and a `ratatui` management UI.

## Build

```bash
cargo build --release
```

## Test

```bash
cargo test
```

## Arch Linux Packaging

This repo includes a reproducible Arch packaging flow for Arch Linux and Arch-based distributions.

Generate the source tarball and package recipe:

```bash
./scripts/build-arch-package.sh
```

That writes these artifacts under `dist/arch/`:

- `plinks-<version>.tar.gz`
- `PKGBUILD`

Build the package with `makepkg`:

```bash
cd dist/arch
makepkg -f
```

Install the resulting package:

```bash
sudo pacman -U plinks-<version>-1-<arch>.pkg.tar.zst
```

Notes:

- the source tarball is generated from the current checkout, excluding ignored files
- package builds run `cargo build --locked --release`
- package checks run `cargo test --locked`
