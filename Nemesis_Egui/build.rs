//! Build script: compiles the Nemesis C++ engine with CMake and links it into the
//! executable (feature `embedded-engine`, Windows/MSVC only).
//!
//! * The CMake build lives in `<repo>/target/nemesis-cmake`, outside Cargo's
//!   per-profile directories, so debug and release builds share one engine build.
//!   Downloaded CMake dependencies go to `<repo>/target/nemesis-deps`.
//! * The engine is always built as `Release` with the dynamic release CRT (`/MD`),
//!   which is what Rust links on MSVC regardless of the Cargo profile.
//! * Both libraries are linked with `+whole-archive`, like the engine's own CLI
//!   does for Havok, because they rely on static registration.
//! * CMake is taken from `NEMESIS_CMAKE`, else the copy bundled with Visual Studio
//!   (located with `vswhere`), else `cmake` on `PATH`. The generator defaults to
//!   CMake's choice (the newest Visual Studio) and can be set with
//!   `NEMESIS_CMAKE_GENERATOR`; the toolset is ClangCL.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Static libraries produced by the CMake build: (CMake source dir, library name).
const LIBRARIES: [(&str, &str); 2] = [
    ("Nemesis_Core_Engine", "Nemesis_Engine_Lib"),
    ("Nemesis_Havok", "Nemesis_Havok_Lib"),
];

/// Inputs that require rebuilding the engine.
const WATCHED: [&str; 8] = [
    "CMakeLists.txt",
    "Nemesis_Core_Engine/CMakeLists.txt",
    "Nemesis_Core_Engine/Python.cmake",
    "Nemesis_Core_Engine/include",
    "Nemesis_Core_Engine/src",
    "Nemesis_Havok/CMakeLists.txt",
    "Nemesis_Havok/include",
    "Nemesis_Havok/src",
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=NEMESIS_CMAKE");
    println!("cargo:rerun-if-env-changed=NEMESIS_CMAKE_GENERATOR");

    if env::var_os("CARGO_FEATURE_EMBEDDED_ENGINE").is_none() {
        return;
    }

    let target = env::var("TARGET").unwrap_or_default();
    if !target.contains("windows-msvc") {
        panic!(
            "the embedded Nemesis engine is only supported on Windows/MSVC (target: {target}); \
             build with `--no-default-features` to use an external Nemesis_Engine executable"
        );
    }

    let manifest =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"));
    let repo = manifest
        .parent()
        .expect("the crate lives inside the repository")
        .to_path_buf();
    let build_dir = repo.join("target").join("nemesis-cmake");
    let deps_dir = repo.join("target").join("nemesis-deps");

    for path in WATCHED {
        println!("cargo:rerun-if-changed={}", repo.join(path).display());
    }

    let cmake = find_cmake();

    if !build_dir.join("CMakeCache.txt").exists() {
        configure(&cmake, &repo, &build_dir, &deps_dir);
    }

    run(
        Command::new(&cmake).arg("--build").arg(&build_dir).args([
            "--config",
            "Release",
            "--target",
            "Nemesis_Engine_Lib",
            "--parallel",
        ]),
        "build the Nemesis engine",
    );

    for (source_dir, library) in LIBRARIES {
        let dir = build_dir.join(source_dir).join("Release");
        println!("cargo:rustc-link-search=native={}", dir.display());
        println!("cargo:rustc-link-lib=static:+whole-archive={library}");
    }
}

/// Configures the CMake build without the Qt launcher and without Python.
fn configure(cmake: &Path, repo: &Path, build_dir: &Path, deps_dir: &Path) {
    let mut command = Command::new(cmake);
    command.arg("-S").arg(repo).arg("-B").arg(build_dir);

    if let Ok(generator) = env::var("NEMESIS_CMAKE_GENERATOR") {
        command.args(["-G", &generator]);
    }

    command
        .args(["-A", "x64", "-T", "ClangCL"])
        .arg("-DNEMESIS_BUILD_APP=OFF")
        .arg("-DNEMESIS_WITH_PYTHON=OFF")
        .arg("-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreadedDLL")
        // Lets MSBuild compile the ~800 sources in parallel.
        .arg("-DCMAKE_CXX_FLAGS_INIT=/MP")
        .arg(format!("-DFETCHCONTENT_BASE_DIR={}", deps_dir.display()));

    run(&mut command, "configure the Nemesis engine with CMake");
}

/// Finds a CMake executable (see the module docs).
fn find_cmake() -> PathBuf {
    if let Some(cmake) = env::var_os("NEMESIS_CMAKE") {
        return PathBuf::from(cmake);
    }

    visual_studio_dir()
        .map(|vs| vs.join(r"Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe"))
        .filter(|cmake| cmake.exists())
        .unwrap_or_else(|| PathBuf::from("cmake"))
}

/// Installation directory of the newest Visual Studio, from `vswhere`.
fn visual_studio_dir() -> Option<PathBuf> {
    let vswhere =
        PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe");
    let output = Command::new(vswhere)
        .args(["-latest", "-products", "*", "-property", "installationPath"])
        .output()
        .ok()?;

    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

/// Runs `command`, failing the build with `what` when it does not succeed.
fn run(command: &mut Command, what: &str) {
    let status = command.status().unwrap_or_else(|err| {
        panic!(
            "failed to {what}: cannot start {:?}: {err}",
            command.get_program()
        )
    });

    assert!(
        status.success(),
        "failed to {what} ({status}); command: {command:?}"
    );
}
