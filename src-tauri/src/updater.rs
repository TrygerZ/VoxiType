//! Lightweight update checker (open-source friendly, no signing required).
//!
//! Queries the GitHub Releases API for the latest tag and compares it to the
//! running version. We deliberately avoid `tauri-plugin-updater` here because
//! release builds are unsigned by default (see the release plan); this checker
//! just notifies the user a newer version exists and links them to it.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

const RELEASES_API: &str = "https://api.github.com/repos/voxitype/voxitype/releases/latest";

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
        .get(RELEASES_API)
        .header("User-Agent", "VoxiType")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        return Err(AppError::network(format!(
            "GitHub releases check failed ({status})"
        )));
    }

    let release: GitHubRelease = serde_json::from_str(&body)?;
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
}
