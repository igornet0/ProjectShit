//! Bakes the GitHub OAuth client id into the build (same sources as the desktop app).
fn main() {
    let client_id = std::env::var("GITHUB_OAUTH_CLIENT_ID")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(read_client_id_from_file)
        .unwrap_or_default();

    println!("cargo:rustc-env=PROJECT_HUB_GITHUB_OAUTH_CLIENT_ID={client_id}");
    println!("cargo:rerun-if-env-changed=GITHUB_OAUTH_CLIENT_ID");
    println!("cargo:rerun-if-changed=../../apps/desktop/github-oauth-client-id");
}

fn read_client_id_from_file() -> Option<String> {
    let content = std::fs::read_to_string("../../apps/desktop/github-oauth-client-id").ok()?;
    content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
}
