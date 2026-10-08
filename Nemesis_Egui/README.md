# Nemesis_Egui

A Rust/[egui](https://github.com/emilk/egui) front-end for the Nemesis Unlimited Behavior Engine.
Its layout follows [D-Merge](https://github.com/SARDONYX-sard/d-merge).

The C++ engine (`Nemesis_Core_Engine` + `Nemesis_Havok`) is compiled by `build.rs` and linked into `Nemesis_Egui.exe` through FFI (`extern "C" nemesis_engine_main`), so the GUI and the engine are a single binary.

To patch, the GUI starts a copy of itself in engine mode:

```
Nemesis_Egui.exe --nemesis-engine <engine home> -p <platform> -pi -d <Skyrim Data> [-o <output>] [-db] [-s] -m <mod codes...>
```

The engine arguments are the same ones the original Qt launcher passed to `Nemesis_Engine.exe`. The *engine home* is `<Skyrim Data>/Nemesis_Engine`. Running the engine in a child process has three benefits:

- each patch starts with fresh engine state (the engine keeps global state);
- **Cancel** can stop the engine immediately, and an engine crash cannot take the GUI down;
- under MO2, the child inherits the virtual file system.

The embedded engine is built without Python, so the optional scripts in `scripts/start` and `scripts/end` are skipped. A single binary cannot ship `python312.dll`. To use a separately built `Nemesis_Engine.exe` with Python, set its path in **Settings**.

## Features

- Pick the mod source and an output directory, for example an MO2 mod folder. The mod source can be:
  - a Skyrim `Data` directory. Under MO2 this is the game's own `Data`, which the virtual file system fills with every enabled mod;
  - a glob over MO2 mod folders, like D-Merge, for example `D:\Skyrim Special Edition\MO2\mods\*`. `*` and `?` match within one path component;
  - an MO2 `mods` folder without `\*`, which is treated as `mods\*`.
- **Empty field = automatic.** The GUI uses the `Data` folder of this user's Skyrim install, i.e. `<folder containing SkyrimSE.exe>/Data`. It finds the install from `gamePath` in MO2's `ModOrganizer.ini` (searched upward from the executable and the working directory), then from a parent folder that contains the game executable, then from the registry. The log names the method that matched.
- Lists every mod found in `<source>/Nemesis_Engine/mod/*/info.ini`, the layout of published Nemesis mods, and in `<engine dir>/mods/*/info.ini`. Each mod has a checkbox and shows its author and site.
- The Skyrim `Data` passed to the engine with `-d` is detected separately. It comes from the registry when the source is a glob, and can be overridden in Settings.
- Drag & drop to reorder, plus right-click → move to top/up/down/bottom.
- **Merge order:** rows are merged from top to bottom. Lower rows are applied later and win conflicts. This matches the original launcher and the engine's `-m` semantics, where the right-most code has the highest priority.
- Live progress bar, colored engine log, cancel, and "Open Log" / "Log (Directory)" buttons.
- Settings: engine path, platform (`-p`), debug (`-db`), synchronous (`-s`), and dark/light theme.
- Settings and mod order are saved to `%APPDATA%\Nemesis_Egui\settings.json`. MO2 does not virtualize this folder, so writes are not redirected to `overwrite`. An old `nemesis_egui.json` next to the executable is migrated automatically.
- On startup and after each rescan, the log lists the executable, the working directory, the settings file, the resolved paths, and every folder that contained mods. Copy this block when reporting detection problems.

## Running under Mod Organizer 2

1. Put `Nemesis_Egui.exe` in its own MO2 mod folder, for example `<MO2>/mods/Nemesis_egui/`, and add it as an MO2 executable.
2. Leave the data field empty to use the game's `Data` folder, which MO2 virtualizes to include every enabled mod. Enter a glob such as `<MO2>/mods/*` only if you want to scan mod folders directly.
3. The GUI never relies on `Path::exists` / `is_file` / `is_dir`, because these can give false negatives inside MO2's virtual file system (USVFS). It opens or lists the path instead. It also collects each directory listing before reading files, so USVFS never sees nested directory handles.

## Source layout

The views in `ui/` only draw borrowed state and return events. `app.rs` owns the state and applies those events.

| Module | Responsibility |
| --- | --- |
| `main.rs` | Window setup and launch |
| `app.rs` | App state, panel layout, handling UI events |
| `config.rs` | Settings saved to `%APPDATA%\Nemesis_Egui\settings.json`, legacy migration |
| `location/` | Resolving the data field: glob expansion (`glob`), MO2 / Skyrim detection (`detect`), `-d` and engine path |
| `diagnostics.rs` | Log lines describing detected paths and scan results |
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

Requirements for the embedded engine (feature `embedded-engine`, on by default):

- Windows with Visual Studio 2022 or later, including the **C++ Clang tools for Windows** component (the engine is compiled with clang-cl).
- CMake. `build.rs` uses `NEMESIS_CMAKE` if set, otherwise the CMake bundled with Visual Studio, otherwise `cmake` on `PATH`. Set `NEMESIS_CMAKE_GENERATOR` to choose a generator.
- Network access on the first build. CMake downloads `nlohmann/json` and `exprtk` into `target/nemesis-deps`.

The first build compiles about 800 C++ files into `target/nemesis-cmake`. Later builds only recompile changed engine sources. Debug and release builds share this engine build, which is always compiled as Release with `/MD`.

To build only the GUI and use an external `Nemesis_Engine.exe` instead, run:

```sh
cargo build --release --no-default-features
```

## Tests

```sh
cargo test
# Optional end-to-end test against a real (or fake) engine executable:
NEMESIS_FAKE_ENGINE=path/to/Nemesis_Engine.exe cargo test fake
```
