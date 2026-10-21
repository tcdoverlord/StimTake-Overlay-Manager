use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=StimTake.rc");
    println!("cargo:rerun-if-changed=assets/StimTake.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") { return; }

    let out_dir = match env::var("OUT_DIR") { Ok(v) => PathBuf::from(v), Err(_) => return };
    let res = out_dir.join("StimTake.res");
    let msvc = env::var("CARGO_CFG_TARGET_ENV").map(|v| v == "msvc").unwrap_or(false);
    let result = if msvc {
        Command::new("rc.exe").args(["/nologo", "/fo"]).arg(&res).arg("StimTake.rc").status()
    } else {
        Command::new("windres").arg("StimTake.rc").arg("-O").arg("coff").arg("-o").arg(&res).status()
    };
    match result {
        Ok(status) if status.success() && res.exists() => {
            println!("cargo:rustc-link-arg-bins={}", res.display());
            println!("cargo:warning=StimTake Windows application icon resource embedded.");
        }
        Ok(status) => println!("cargo:warning=Icon resource compiler exited with {status}; installer and shortcut icons will still use StimTake.ico."),
        Err(err) => println!("cargo:warning=Could not run Windows resource compiler ({err}); installer and shortcut icons will still use StimTake.ico."),
    }
}
