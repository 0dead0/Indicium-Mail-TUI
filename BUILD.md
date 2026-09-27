# Build Indicium Mail on this machine

Ubuntu **24.04** WSL2. Target binary: `imt` (Rust **1.80+**).

## Prerequisites

- `gcc`, `libssl-dev` (usually present)
- `pkg-config` recommended: `sudo apt install pkg-config`
- Rust via rustup (stable; e.g. 1.98.x)

If `pkg-config` is missing, set OpenSSL paths for the build:

```bash
export OPENSSL_DIR=/usr
export OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
export OPENSSL_INCLUDE_DIR=/usr/include
```

## Build / install

```bash
cd ~/projects/Indicium-Mail-TUI
cargo install --path crates/imt --locked
```

Installs to `~/.cargo/bin/imt` (first build ~several minutes). Ensure that directory is on `PATH` (`which imt`).

## Rebuild after local changes

```bash
cd ~/projects/Indicium-Mail-TUI
cargo install --path crates/imt --locked
```

Runtime only needs the binary; the Rust toolchain is needed for rebuilds.

## Tests

See [TESTS.md](TESTS.md) (`./manage.sh test`).

## Disk (approx., this machine)

| Path | Size |
|------|------|
| `~/.rustup` | ~1.5 GB |
| `~/.cargo` (incl. `imt` ~12 MB) | ~324 MB |
| Source tree | ~14 MB |
| Cargo target cache (disposable) | ~800+ MB |
