#pragma once

// Entry point of the Nemesis engine, callable from other programs (e.g. the Rust
// GUI through FFI). It behaves exactly like the command line executable:
// `argv[0]` must be the engine executable path, because the engine resolves its
// resources (`mods`, `behavior_templates`, `alternate_animations`, ...) relative
// to that directory. Output is written to stdout. Returns the process exit code;
// exceptions never escape.

#ifdef _WIN32
using nemesis_engine_char = wchar_t;
#else
using nemesis_engine_char = char;
#endif

extern "C" int nemesis_engine_main(int argc, nemesis_engine_char* argv[]);
