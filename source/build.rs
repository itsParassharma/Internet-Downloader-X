//! Embed the selected application icon and IDX version information without build dependencies.
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn resource_text(icon: &Path, version: &str, binary: &str, description: &str) -> String {
    let mut numbers: Vec<&str> = version.split(['.', '-']).take(3).collect();
    while numbers.len() < 3 {
        numbers.push("0");
    }
    let numeric_version = format!("{},{},{},0", numbers[0], numbers[1], numbers[2]);
    let icon_path = icon
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let manifest_path = icon
        .with_file_name("idx.manifest")
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!(
        r#"#pragma code_page(65001)
101 ICON "{icon_path}"
1 24 "{manifest_path}"
1 VERSIONINFO
FILEVERSION {numeric_version}
PRODUCTVERSION {numeric_version}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "IDX Contributors\0"
      VALUE "FileDescription", "{description}\0"
      VALUE "FileVersion", "{version}\0"
      VALUE "InternalName", "{binary}\0"
      VALUE "LegalCopyright", "Copyright (c) 2026 IDX Contributors. MIT License.\0"
      VALUE "OriginalFilename", "{binary}.exe\0"
      VALUE "ProductName", "IDX - Internet Downloader X\0"
      VALUE "ProductVersion", "{version}\0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x0409, 1200
  END
END
"#
    )
}

fn find_gnu_compiler(target: &str) -> PathBuf {
    for variable in ["IDX_WINDRES", "WINDRES"] {
        if let Some(path) = env::var_os(variable).filter(|value| !value.is_empty()) {
            return path.into();
        }
    }
    let prefix = target.split('-').next().unwrap_or("x86_64");
    let candidates = [
        format!("{prefix}-w64-mingw32-windres"),
        "llvm-windres".into(),
        "windres".into(),
    ];
    for candidate in candidates {
        if Command::new(&candidate)
            .arg("--version")
            .output()
            .is_ok_and(|result| result.status.success())
        {
            return candidate.into();
        }
    }
    panic!(
        "Windows resources require windres. Put the LLVM-MinGW bin directory on PATH or set IDX_WINDRES to the full windres executable path."
    );
}

fn find_msvc_resource_compiler() -> PathBuf {
    for variable in ["IDX_RC", "RC"] {
        if let Some(path) = env::var_os(variable).filter(|value| !value.is_empty()) {
            return path.into();
        }
    }
    if env::split_paths(&env::var_os("PATH").unwrap_or_default())
        .any(|path| path.join("rc.exe").is_file())
    {
        return "rc.exe".into();
    }
    // Cargo discovers the MSVC linker outside a Developer Command Prompt, but
    // that does not place the Windows SDK resource compiler on this process's PATH.
    let mut roots = Vec::new();
    if let Some(root) = env::var_os("WindowsSdkDir") {
        roots.push(PathBuf::from(root).join("bin"));
    }
    if let Some(root) = env::var_os("ProgramFiles(x86)") {
        roots.push(PathBuf::from(root).join("Windows Kits/10/bin"));
    }
    for root in roots {
        let mut versions: Vec<_> = fs::read_dir(&root)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.path())
            .collect();
        versions.sort();
        for version in versions.into_iter().rev() {
            for arch in ["x64", "x86"] {
                let compiler = version.join(arch).join("rc.exe");
                if compiler.is_file() {
                    return compiler;
                }
            }
        }
    }
    "rc.exe".into()
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/idx.ico");
    println!("cargo:rerun-if-changed=assets/idx.manifest");
    for variable in [
        "IDX_WINDRES",
        "WINDRES",
        "IDX_RC",
        "RC",
        "PATH",
        "WindowsSdkDir",
        "ProgramFiles(x86)",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    let target = env::var("TARGET").expect("Cargo did not provide TARGET");
    if !target.contains("windows") {
        return;
    }
    let root =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Missing manifest directory"));
    let icon = root.join("assets/idx.ico");
    assert!(
        icon.is_file(),
        "Missing application icon: {}",
        icon.display()
    );
    let output_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo did not provide OUT_DIR"));
    let version = env::var("CARGO_PKG_VERSION").expect("Cargo did not provide package version");
    let gnu = target.ends_with("-gnu");
    let compiler = if gnu {
        find_gnu_compiler(&target)
    } else {
        find_msvc_resource_compiler()
    };

    for (binary, description) in [
        ("idx", "IDX - Internet Downloader X"),
        ("idx-native-host", "IDX - Browser Integration Host"),
    ] {
        let script = output_dir.join(format!("{binary}.rc"));
        let resource = output_dir.join(format!(
            "{binary}-resource.{}",
            if gnu { "o" } else { "res" }
        ));
        let mut resource_source = resource_text(&icon, &version, binary, description);
        if binary == "idx" {
            for id in [272, 273, 274, 275, 276, 277, 278, 293, 295, 331] {
                let relative = format!("assets/native/icon_{id}.ico");
                println!("cargo:rerun-if-changed={relative}");
                let path = root.join(relative).to_string_lossy().replace('\\', "\\\\");
                resource_source.push_str(&format!("\n{id} ICON \"{path}\"\n"));
            }
        }
        fs::write(&script, resource_source).expect("Unable to write Windows resource source");
        let mut command = Command::new(&compiler);
        if gnu {
            command
                .arg("--input")
                .arg(&script)
                .arg("--output")
                .arg(&resource)
                .arg("--output-format=coff");
            match target.split('-').next() {
                Some("x86_64") => {
                    command.arg("--target=pe-x86-64");
                }
                Some("i686") => {
                    command.arg("--target=pe-i386");
                }
                _ => {}
            }
        } else {
            command
                .arg("/nologo")
                .arg("/fo")
                .arg(&resource)
                .arg(&script);
        }
        let result = command.output().unwrap_or_else(|error| {
            panic!("Cannot run Windows resource compiler {}: {error}. Install matching Windows build tools or set IDX_WINDRES (GNU) / IDX_RC (MSVC).", compiler.display())
        });
        assert!(
            result.status.success(),
            "Windows resource compilation failed for {binary}:\n{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            resource.is_file(),
            "Resource compiler produced no resource for {binary}"
        );
        println!("cargo:rustc-link-arg-bin={binary}={}", resource.display());
    }
}
