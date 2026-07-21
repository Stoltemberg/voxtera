#[path = "src/target_contract.rs"]
mod target_contract;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS")
        .expect("Cargo did not provide CARGO_CFG_TARGET_OS to the Voxtera Launcher build");
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH")
        .expect("Cargo did not provide CARGO_CFG_TARGET_ARCH to the Voxtera Launcher build");

    assert!(
        target_contract::is_supported_target(&target_os, &target_arch),
        "Voxtera Launcher supports only Windows 10/11 x64 targets; got {target_os}/{target_arch}"
    );

    tauri_build::build()
}
