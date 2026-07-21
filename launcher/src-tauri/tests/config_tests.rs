use std::path::{Path, PathBuf};

use launcher_core::{LauncherConfig, LauncherPaths};

fn fixture_paths(base: &Path) -> LauncherPaths {
    let root = base.join("Voxtera");

    LauncherPaths {
        config_file: root.join("launcher.json"),
        logs_dir: root.join("logs"),
        cache_dir: root.join("cache"),
        default_install_dir: root.join("game"),
        legacy_config: base.join("launcher").join("voxtera_config.json"),
        root,
    }
}

#[test]
fn migrates_legacy_config_without_deleting_it() {
    let temp = tempfile::tempdir().unwrap();
    let paths = fixture_paths(temp.path());
    std::fs::create_dir_all(paths.legacy_config.parent().unwrap()).unwrap();
    std::fs::write(
        &paths.legacy_config,
        r#"{"install_dir":"D:\\Games\\Voxtera","installed_version":"v0.2.3"}"#,
    )
    .unwrap();

    let config = LauncherConfig::load_or_migrate(&paths).unwrap();

    assert_eq!(config.install_dir, PathBuf::from(r"D:\Games\Voxtera"));
    assert_eq!(config.installed_version.as_deref(), Some("v0.2.3"));
    assert!(paths.legacy_config.exists());
    assert!(paths.config_file.exists());
}

#[test]
fn invalid_install_path_cannot_escape_local_data_by_default() {
    let paths = fixture_paths(Path::new(r"C:\Local"));

    let config = LauncherConfig::default_for(&paths);

    assert_eq!(config.install_dir, paths.default_install_dir);
    assert!(config.install_dir.ends_with(r"Voxtera\game"));
}

#[test]
fn save_atomic_replaces_an_existing_config() {
    let temp = tempfile::tempdir().unwrap();
    let paths = fixture_paths(temp.path());
    let first = LauncherConfig::default_for(&paths);
    first.save_atomic(&paths).unwrap();

    let mut second = first.clone();
    second.installed_version = Some("v0.3.0".to_owned());
    second.start_minimized = true;
    second.save_atomic(&paths).unwrap();

    let saved: LauncherConfig =
        serde_json::from_slice(&std::fs::read(&paths.config_file).unwrap()).unwrap();
    assert_eq!(saved, second);
    assert!(!paths.root.join("launcher.json.tmp").exists());
}
