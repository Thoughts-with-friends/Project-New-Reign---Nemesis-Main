# xtask

Development tasks for this workspace, following the [cargo-xtask](https://github.com/matklad/cargo-xtask) pattern.
The `cargo xtask` alias is defined in [`.cargo/config.toml`](../.cargo/config.toml), so you can run every task from anywhere in the repository.

## Tasks

| Task | Cargo command it runs | What it does |
| --- | --- | --- |
| `cargo xtask fmt` | `cargo +nightly fmt --all` | Formats every crate in the workspace with nightly rustfmt |
| `cargo xtask lint:fix` | `cargo clippy --workspace --fix --allow-staged --allow-dirty` | Applies clippy's automatic fixes to the whole workspace, even with uncommitted changes |
| `cargo xtask build` | `cargo build --release` | Builds the GUI in release mode (`target/release/Nemesis_Egui.exe`) |

`lint-fix` is accepted as an alias for `lint:fix`.

Arguments after the task name are appended to the cargo command:

```sh
cargo xtask build --locked      # -> cargo build --release --locked
```

Each task prints the command it runs before running it:

```text
xtask: > cargo +nightly fmt --all
```

Run `cargo xtask help` to list the tasks.

## Requirements

- `fmt` needs the nightly toolchain with rustfmt:

  ```sh
  rustup toolchain install nightly --component rustfmt
  ```

- xtask calls the `cargo` on `PATH` (the rustup proxy) instead of `$CARGO`, because only the proxy understands `+nightly`.

## Adding a task

Add an entry to `TASKS` in [`src/main.rs`](src/main.rs):

```rust
Task {
    name: "test",
    aliases: &[],
    description: "Run all tests",
    args: &["test", "--workspace"],
},
```
