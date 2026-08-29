fn main() {
    tauri_build::build();

    let client_id = std::env::var("GITHUB_OAUTH_CLIENT_ID")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(read_client_id_from_file)
        .unwrap_or_default();

    println!(
        "cargo:rustc-env=PROJECT_HUB_GITHUB_OAUTH_CLIENT_ID={client_id}"
    );
    println!("cargo:rerun-if-env-changed=GITHUB_OAUTH_CLIENT_ID");
    println!("cargo:rerun-if-changed=github-oauth-client-id");
}

fn read_client_id_from_file() -> Option<String> {
    let content = std::fs::read_to_string("github-oauth-client-id").ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        return Some(line.to_string());
    }
    None
}
