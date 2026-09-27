# Running tests

Use the normal Rust toolchain (`cargo` via rustup). No separate test toolchain.

## Preferred

```bash
./manage.sh test
```

That sets OpenSSL paths when needed (see below) and runs `cargo test --workspace`.

Pass-through args work:

```bash
./manage.sh test -p imt-tui --lib
./manage.sh test -p imt-tui enter_on_collapsed_account_expands_it
./manage.sh test -- --nocapture
```

## Direct cargo

```bash
cargo test --workspace
cargo test -p imt-tui --lib
```

## OpenSSL on this machine (WSL Ubuntu)

`libssl-dev` is present, but **`pkg-config` is often missing**. Without it, `openssl-sys` fails to link.

**Fix A (recommended once):**

```bash
sudo apt install pkg-config
```

**Fix B (no install):** export before `cargo test` / `cargo build` (same as `BUILD.md`):

```bash
export OPENSSL_DIR=/usr
export OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
export OPENSSL_INCLUDE_DIR=/usr/include
```

`./manage.sh test` applies Fix B automatically when `pkg-config` is not on `PATH`.

## Where tests live

| Crate | Kind | Location |
|---|---|---|
| `imt-tui` | unit | `crates/imt-tui/src/**` (`#[cfg(test)]`, e.g. `app::sidebar_nav_tests`) |
| other crates | unit / integration | under each `crates/<name>/` as usual |

TUI behavior tests typically use `InMemoryDataSource::sample()` and `App::dispatch(KeyAction::…)`.

## Agent / CI note

Cursor’s sandbox may redirect `CARGO_TARGET_DIR` under `/tmp/cursor-sandbox-cache/…`. That is still the same compiler; it is not a second toolchain. Prefer `./manage.sh test` from the repo root so OpenSSL env is consistent.
