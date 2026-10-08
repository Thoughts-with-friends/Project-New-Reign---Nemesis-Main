# Nemesis_Egui

A Rust/[egui](https://github.com/emilk/egui) front-end for the Nemesis Unlimited Behavior Engine.
Its layout follows [D-Merge](https://github.com/SARDONYX-sard/d-merge).

The GUI does not link the C++ engine. It runs `Nemesis_Engine(.exe)` as a child process, the same way the Qt launcher (`Nemesis_App`) does:

```
Nemesis_Engine -p <platform> -pi [-d <data>] [-o <output>] [-db] [-s] -m <mod codes...>
```

## Features

- Pick the Skyrim `Data` directory and an output directory, for example an MO2 mod folder.
- Lists every mod in `<engine dir>/mods/*/info.ini`, each with a checkbox. It also shows the author and site.
- Drag & drop to reorder, plus right-click → move to top/up/down/bottom.
- **Merge order:** rows are merged from top to bottom. Lower rows are applied later and win conflicts. This matches the original launcher and the engine's `-m` semantics, where the right-most code has the highest priority.
- Live progress bar, colored engine log, cancel, and "Open Log" / "Log (Directory)" buttons.
- Settings: engine path, platform (`-p`), debug (`-db`), synchronous (`-s`), and dark/light theme.
- Settings and mod order are saved to `nemesis_egui.json` next to the executable.

## Source layout

The views in `ui/` only draw borrowed state and return events. `app.rs` owns the state and applies those events.

| Module | Responsibility |
| --- | --- |
| `main.rs` | Window setup and launch |
| `app.rs` | App state, panel layout, handling UI events |
| `config.rs` | Settings saved to `nemesis_egui.json` |
| `os.rs` | Opening files, folders and web links |
| `engine/` | Engine command line (`request`), output parsing (`output`), child process (`process`), `log.txt` location (`log_path`) |
| `mods/` | `info.ini` scanning (`scan`), merge order (`order`), editable mod list (`list`) |
| `session/` | Patch run lifecycle and outcome (`mod`), bounded log buffer (`log`) |
| `ui/` | Style, widgets, tab bar, directory inputs, log view, action bar, settings page |
| `ui/mod_table/` | Column layout, header, rows, drag & drop |

## Build

The repository root is a Cargo workspace, so these commands work from the root or from `Nemesis_Egui/`:

```sh
cargo run                # build and start the GUI (debug)
cargo build --release    # target/release/Nemesis_Egui.exe
```

By default the engine is expected at `<Data>/nemesis_engine/Nemesis_Engine.exe`. You can change this under **Settings**.

## Tests

```sh
cargo test
# Optional end-to-end test against a real (or fake) engine executable:
NEMESIS_FAKE_ENGINE=path/to/Nemesis_Engine.exe cargo test fake
```
