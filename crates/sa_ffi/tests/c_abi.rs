//! Builds `tests/ffi/main.c` against `include/sa.h` and the static library,
//! then runs it. With `SA2_DIR` set it also exercises a real install.
//! Windows/MSVC only (the target this library ships for first).
#![cfg(all(windows, target_env = "msvc"))]

use std::path::PathBuf;
use std::process::Command;

#[test]
fn c_program_links_and_runs() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Not canonicalize(): cl.exe rejects `\\?\` verbatim paths.
    let root = crate_dir.parent().unwrap().parent().unwrap().to_path_buf();
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };

    // Make sure the staticlib for this profile exists.
    let mut build = Command::new(env!("CARGO"));
    build.current_dir(&root).args(["build", "-q", "-p", "sa_ffi"]);
    if profile == "release" {
        build.arg("--release");
    }
    assert!(build.status().unwrap().success(), "cargo build -p sa_ffi failed");

    let target = std::env::var("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|_| root.join("target"));
    let lib = target.join(profile).join("sa_ffi.lib");
    assert!(lib.is_file(), "missing {}", lib.display());
    let out_dir = target.join(profile).join("ffi-test");
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join("sa_ffi_test.exe");

    let arch = if cfg!(target_arch = "x86") { "i686-pc-windows-msvc" } else { "x86_64-pc-windows-msvc" };
    let compiler = cc::Build::new().target(arch).host(arch).opt_level(0).cargo_metadata(false).get_compiler();
    let mut cmd = compiler.to_command();
    cmd.current_dir(&out_dir)
        .arg("/nologo")
        .arg(format!("/I{}", crate_dir.join("include").display()))
        .arg(root.join("tests/ffi/main.c"))
        .arg(format!("/Fe{}", exe.display()))
        .arg("/link")
        .arg(&lib)
        .args(["kernel32.lib", "ntdll.lib", "userenv.lib", "ws2_32.lib", "dbghelp.lib", "advapi32.lib", "bcrypt.lib"]);
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "C build failed:\n{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));

    let mut run = Command::new(&exe);
    if let Ok(dir) = std::env::var("SA2_DIR") {
        run.arg(dir);
    }
    let out = run.output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    println!("{stdout}");
    assert!(out.status.success(), "C program failed:\n{stdout}\n{}", String::from_utf8_lossy(&out.stderr));
    assert!(stdout.contains("ALL OK"));
}
