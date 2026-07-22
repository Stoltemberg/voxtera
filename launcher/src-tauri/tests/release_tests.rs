use launcher_core::{GitHubAsset, GitHubRelease, select_release};

fn load_fixture() -> Vec<GitHubRelease> {
    serde_json::from_str(include_str!("fixtures/releases.json")).unwrap()
}

fn fixture_with_duplicate(name: &str) -> Vec<GitHubRelease> {
    let mut releases = load_fixture();
    let selected = releases
        .iter_mut()
        .find(|release| release.tag_name == "v0.2.3")
        .unwrap();
    let duplicate = selected
        .assets
        .iter()
        .find(|asset| asset.name == name)
        .unwrap()
        .clone();
    selected.assets.push(duplicate);
    releases
}

#[test]
fn selects_newest_non_draft_preview_with_exact_assets() {
    let selected = select_release(&load_fixture()).unwrap();

    assert_eq!(selected.version.to_string(), "0.2.3");
    assert!(
        selected
            .game_archive
            .url
            .ends_with("Voxtera-windows-x64.zip")
    );
    assert!(selected.manifest.url.ends_with("voxtera-manifest.json"));
    assert!(
        selected
            .launcher_installer
            .url
            .ends_with("VoxteraLauncher-setup.exe")
    );
}

#[test]
fn rejects_duplicate_launcher_assets() {
    let releases = fixture_with_duplicate("VoxteraLauncher-setup.exe");

    assert_eq!(
        select_release(&releases).unwrap_err().code(),
        "release_contract"
    );
}

#[test]
fn rejects_a_preview_missing_a_required_asset() {
    let mut releases = load_fixture();
    releases
        .iter_mut()
        .find(|release| release.tag_name == "v0.2.3")
        .unwrap()
        .assets
        .retain(|asset| asset.name != "voxtera-manifest.json");

    assert_eq!(
        select_release(&releases).unwrap_err().code(),
        "release_contract"
    );
}

#[test]
fn rejects_a_malformed_non_draft_version() {
    let mut releases = load_fixture();
    releases.push(GitHubRelease {
        tag_name: "version-next".to_owned(),
        draft: false,
        prerelease: true,
        assets: Vec::new(),
    });

    assert_eq!(
        select_release(&releases).unwrap_err().code(),
        "release_contract"
    );
}

#[test]
fn rejects_non_https_asset_urls() {
    let releases = vec![GitHubRelease {
        tag_name: "v0.2.3".to_owned(),
        draft: false,
        prerelease: true,
        assets: vec![
            GitHubAsset {
                name: "VoxteraLauncher-setup.exe".to_owned(),
                size: 1,
                browser_download_url: "http://example.invalid/launcher.exe".to_owned(),
            },
            GitHubAsset {
                name: "Voxtera-windows-x64.zip".to_owned(),
                size: 1,
                browser_download_url: "https://example.invalid/game.zip".to_owned(),
            },
            GitHubAsset {
                name: "voxtera-manifest.json".to_owned(),
                size: 1,
                browser_download_url: "https://example.invalid/manifest.json".to_owned(),
            },
        ],
    }];

    assert_eq!(
        select_release(&releases).unwrap_err().code(),
        "release_contract"
    );
}
