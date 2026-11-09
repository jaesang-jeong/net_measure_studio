fn main() {
    println!("cargo:rerun-if-changed=resource.rc");
    println!("cargo:rerun-if-changed=nxs.ICO");
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=app_icon.res");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let root_res = format!("{}\\app_icon.res", manifest_dir);

        if std::path::Path::new(&root_res).exists() {
            println!("cargo:rustc-link-arg={}", root_res);
        } else {
            let out_dir = std::env::var("OUT_DIR").unwrap();
            let out_res = format!("{}\\app_icon.res", out_dir);
            if let Ok(status) = std::process::Command::new("rc.exe")
                .args(["/fo", &out_res, "resource.rc"])
                .status()
            {
                if status.success() {
                    println!("cargo:rustc-link-arg={}", out_res);
                }
            }
        }
    }
}
