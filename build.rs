use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=TREETOP_CLI_BUILD_CHANNEL");
    println!("cargo:rerun-if-env-changed=TREETOP_CLI_BUILD_GIT_SHA");
    println!("cargo:rerun-if-env-changed=TREETOP_CLI_BUILD_TIMESTAMP");

    let package_version = env::var("CARGO_PKG_VERSION").expect("Cargo package version");
    let channel = env::var("TREETOP_CLI_BUILD_CHANNEL").unwrap_or_default();
    let git_sha = env::var("TREETOP_CLI_BUILD_GIT_SHA").unwrap_or_default();
    let short_sha = git_sha
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(12)
        .collect::<String>();

    let version = if channel == "main" && !short_sha.is_empty() {
        format!("v{package_version}+main.g{short_sha}")
    } else {
        format!("v{package_version}")
    };

    println!("cargo:rustc-env=TREETOP_CLI_VERSION={version}");
    println!(
        "cargo:rustc-env=TREETOP_CLI_BUILD_TIMESTAMP={}",
        env::var("TREETOP_CLI_BUILD_TIMESTAMP").unwrap_or_else(|_| "unknown".to_string())
    );
    println!(
        "cargo:rustc-env=TREETOP_CLI_GIT_DESCRIBE={}",
        if short_sha.is_empty() {
            "unknown"
        } else {
            &short_sha
        }
    );
    println!(
        "cargo:rustc-env=TREETOP_CLI_BUILD_TARGET={}",
        env::var("TARGET").expect("Cargo build target")
    );
}
