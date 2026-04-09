# plinks

`plinks` is a project-local link manager for repositories. It keeps shared links in a checked-in `project-links.toml` file so everyone on the project can open the same docs, dashboards, tickets, and runbooks from either a CLI or a `ratatui` interface.

## Why

- Keep useful project links in the repo instead of in browser bookmarks.
- Open links by a stable short name or alias.
- Group related links with tags.
- Manage the file directly from the terminal with shell commands or an interactive TUI.

## Install

Prebuilt binaries are published on GitHub Releases for these targets:

- `x86_64-pc-windows-msvc`
- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`

Each asset is target-specific. Windows, Linux, and macOS binaries are not interchangeable.

Windows releases are unsigned portable `.zip` archives containing `plinks.exe`, `LICENSE`, and `README.md`. Linux and macOS releases are `.tar.gz` archives containing `plinks`, `LICENSE`, and `README.md`. Depending on local policy, Windows may show SmartScreen or other trust warnings before first launch.

Build from source:

```bash
cargo install --path .
```

Or run it from the checkout:

```bash
cargo run -- <command>
```

## Usage

Add a link:

```bash
plinks add docs https://docs.rs --alias api --tag rust --tag reference --note "Rust API docs"
```

List links:

```bash
plinks list
plinks list --tag rust
```

Open a link by primary name or alias:

```bash
plinks open docs
plinks open api
```

Open every link with a tag:

```bash
plinks open --tag rust
```

Launch the interactive TUI:

```bash
plinks manage
```

## How `plinks` finds `project-links.toml`

`plinks` looks for `project-links.toml` in the current directory first.

If it does not find one, it checks ancestor directories up to the Git repository root:

- If an ancestor already contains `project-links.toml`, that file is used.
- If no file exists yet, `plinks` uses `<git-root>/project-links.toml`.
- Outside a Git repository, it falls back to `<cwd>/project-links.toml`.

This makes it practical to run `plinks` anywhere inside a repository while still keeping one shared config file at the project level.

## Config format

The config file format is schema version `1`:

```toml
version = 1

[links]

[links.docs]
url = "https://docs.rs"
aliases = ["api"]
tags = ["reference", "rust"]
note = "Rust API docs"

[links.jira]
url = "https://jira.example.com/browse/PROJ"
tags = ["ops"]
```

Primary names, aliases, and tags are normalized to lowercase and may contain letters, numbers, `_`, and `-`.

## Development

Build:

```bash
cargo build --release
```

Test:

```bash
cargo test
```

## Releases

GitHub Releases publish prebuilt binaries for Windows, Linux, and macOS. Release assets are named as stable target-specific archives:

- `plinks-v<version>-x86_64-pc-windows-msvc.zip`
- `plinks-v<version>-x86_64-unknown-linux-gnu.tar.gz`
- `plinks-v<version>-x86_64-apple-darwin.tar.gz`

Every release also includes a `SHA256SUMS` file covering all published archives.

## Maintainer Release Process

1. Bump the crate version in `Cargo.toml` and refresh `Cargo.lock` so locked CI builds stay in sync.
2. Merge the release commit to `main`.
3. Create and push a matching Git tag in the form `vX.Y.Z`.
4. GitHub Actions validates that the tag matches `Cargo.toml`, builds the release binaries, runs `--help` smoke tests for each release target, packages the binary together with `LICENSE` and `README.md`, generates `SHA256SUMS`, and publishes the release assets automatically.

Arch packaging remains a separate distribution path and is still generated with `./scripts/build-arch-package.sh`.

## Arch Linux Packaging

Generate Arch packaging artifacts:

```bash
./scripts/build-arch-package.sh
```

This writes the source tarball and `PKGBUILD` to `dist/arch/`.

Build the package with `makepkg`:

```bash
cd dist/arch
makepkg -f
```

## License

MIT. See [LICENSE](LICENSE).
