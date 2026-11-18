//! Gives popamp.exe its icon on Windows, for Explorer, shortcuts and the
//! taskbar, and sets `linux_like` for Linux and the other Unix systems
//! with X11, ALSA and the freedesktop services: every Unix but macOS,
//! which is a Unix too but has none of them. It also puts popampdeluxe.txt,
//! what is new, beside the built program, as the downloads have it.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(linux_like)");
    let unix = std::env::var("CARGO_CFG_TARGET_FAMILY").is_ok_and(|f| f.split(',').any(|f| f == "unix"));
    if unix && std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        println!("cargo::rustc-cfg=linux_like");
    }
    // Beside the program, in target/<profile>: OUT_DIR is target/<profile>/build/<package>/<hash>/out.
    // Only a convenience, so a failure is no error.
    println!("cargo:rerun-if-changed=../../popampdeluxe.txt");
    if let Some(profile) =
        std::env::var_os("OUT_DIR").map(std::path::PathBuf::from).and_then(|d| d.ancestors().nth(4).map(|p| p.to_path_buf()))
    {
        let _ = std::fs::copy("../../popampdeluxe.txt", profile.join("popampdeluxe.txt"));
    }
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=../../assets/logo/popamp.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/logo/popamp.ico");
        res.compile().expect("compile the Windows resources");
    }
}
