fn main() {
    let version = std::process::Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "dev".to_string());
    println!("cargo:rustc-env=CZOR_VERSION={}", version.trim());
}
