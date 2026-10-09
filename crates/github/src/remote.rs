use std::path::Path;

use project_hub_git::GitRepository;

pub fn remote_urls_for_path(path: &Path) -> Vec<String> {
    let Ok(git) = GitRepository::open(path) else {
        return Vec::new();
    };

    git.origin_url().into_iter().collect()
}

pub fn parse_github_full_name(url: &str) -> Option<String> {
    let url = url.trim().trim_end_matches(".git");

    if let Some(rest) = url.strip_prefix("https://github.com/") {
        return Some(rest.to_string());
    }
    if let Some(rest) = url.strip_prefix("http://github.com/") {
        return Some(rest.to_string());
    }
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        return Some(rest.to_string());
    }
    if let Some(rest) = url.strip_prefix("ssh://git@github.com/") {
        return Some(rest.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_https_remote() {
        assert_eq!(
            parse_github_full_name("https://github.com/octocat/Hello-World.git"),
            Some("octocat/Hello-World".into())
        );
    }

    #[test]
    fn parses_ssh_remote() {
        assert_eq!(
            parse_github_full_name("git@github.com:octocat/Hello-World.git"),
            Some("octocat/Hello-World".into())
        );
    }
}
