pub(crate) fn is_supported_target(target_os: &str, target_arch: &str) -> bool {
    target_os == "windows" && target_arch == "x86_64"
}

#[cfg(test)]
mod tests {
    use super::is_supported_target;

    #[test]
    fn accepts_windows_x86_64() {
        assert!(is_supported_target("windows", "x86_64"));
    }

    #[test]
    fn rejects_non_windows_targets() {
        assert!(!is_supported_target("linux", "x86_64"));
    }

    #[test]
    fn rejects_non_x86_64_targets() {
        assert!(!is_supported_target("windows", "aarch64"));
    }
}
