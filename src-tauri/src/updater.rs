//! Lightweight update checker (open-source friendly, no signing required).
//!
//! Queries the GitHub Releases API for the latest tag and compares it to the
//! running version. We deliberately avoid `tauri-plugin-updater` here because
//! release builds are unsigned by default (see the release plan); this checker
//! just notifies the user a newer version exists and links them to it.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

/// GitHub repository that hosts VoxiType releases. Must match the `origin`
/// remote; a 404 from this endpoint means "no published release yet", which we
/// treat as "no update" rather than a hard failure.
const REPO: &str = "TrygerZ/VoxiType";

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub notes: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// Check GitHub for a newer release. `current` is the running app version.
pub async fn check(current: &str) -> Result<UpdateInfo> {
    let client = crate::util::http_client();
    let resp = client
        .get(format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .header("User-Agent", "VoxiType")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    let status = resp.status().as_u16();
    let body = resp.text().await?;
    interpret(status, &body, current)
}

/// "No published release to compare against" — neither an update nor an error.
/// Used when GitHub has no latest release (404), the normal state before the
/// first public tag ships.
fn no_update(current: &str) -> UpdateInfo {
    UpdateInfo {
        available: false,
        current_version: current.to_string(),
        latest_version: current.to_string(),
        notes: String::new(),
        url: String::new(),
    }
}

/// Classify the GitHub response. A 404 (repo has no published release yet, or
/// the repo is misconfigured) is reported as "no update" instead of a hard
/// network error — the background check is advisory, not a reason to crash.
/// 200 parses the release; anything else is a real failure.
fn interpret(status: u16, body: &str, current: &str) -> Result<UpdateInfo> {
    if status == 404 {
        return Ok(no_update(current));
    }
    if status != 200 {
        return Err(AppError::network(format!(
            "GitHub releases check failed ({status})"
        )));
    }

    let release: GitHubRelease = serde_json::from_str(body)?;
    let latest = release.tag_name.trim_start_matches('v').to_string();
    let available = !release.draft && !release.prerelease && is_newer(&latest, current);

    Ok(UpdateInfo {
        available,
        current_version: current.to_string(),
        latest_version: latest,
        notes: release.body,
        url: release.html_url,
    })
}

/// Compare dotted numeric versions: is `latest` strictly newer than `current`?
fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|p| {
                p.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0)
            })
            .collect()
    };
    let (a, b) = (parse(latest), parse(current));
    let n = a.len().max(b.len());
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_comparison() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        // Tolerates suffixes like "1.2.3-beta".
        assert!(is_newer("0.2.0-beta", "0.1.0"));
    }

    #[test]
    fn interpret_404_is_not_an_error() {
        let info = interpret(404, r#"{"message":"Not Found"}"#, "0.4.2").unwrap();
        assert!(!info.available);
        assert_eq!(info.current_version, "0.4.2");
        assert_eq!(info.latest_version, "0.4.2");
    }

    #[test]
    fn interpret_200_parses_release() {
        let body = r#"{"tag_name":"v0.5.0","body":"notes","html_url":"https://x","draft":false,"prerelease":false}"#;
        let info = interpret(200, body, "0.4.2").unwrap();
        assert!(info.available);
        assert_eq!(info.latest_version, "0.5.0");
        assert_eq!(info.url, "https://x");
        assert_eq!(info.notes, "notes");
    }

    #[test]
    fn interpret_draft_release_is_not_available() {
        let body = r#"{"tag_name":"v0.5.0","draft":true,"prerelease":false}"#;
        let info = interpret(200, body, "0.4.2").unwrap();
        assert!(!info.available);
    }

    #[test]
    fn interpret_server_error_is_error() {
        assert!(interpret(503, "oops", "0.4.2").is_err());
    }
}
