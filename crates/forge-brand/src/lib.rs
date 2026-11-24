#![forbid(unsafe_code)]

//! Generated user-visible brand constants from `branding/brand.toml` (Blueprint 6.1).

include!(concat!(env!("OUT_DIR"), "/brand_constants.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::hint::black_box;

    #[test]
    fn brand_constants_are_non_empty() {
        assert!(!black_box(NAME).is_empty());
        assert!(!black_box(ID).is_empty());
        assert!(!black_box(PRETTY_NAME).is_empty());
        assert!(black_box(HOME_URL).starts_with("https://"));
        assert!(black_box(ACCENT).starts_with('#'));
        assert!(black_box(RADIUS_PX) > 0);
    }

    #[test]
    fn upstream_lineage_preserved_n6() {
        // Non-negotiable N6: ID_LIKE stays machine-readable.
        assert_eq!(black_box(ID_LIKE), "debian");
        assert!(black_box(UPSTREAM_DOC).contains("debian.org"));
    }

    #[test]
    fn generated_os_release_contains_required_fields() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/brand/os-release");
        let text =
            std::fs::read_to_string(&path).expect("target/brand/os-release must exist after build");
        for required in [
            "NAME=",
            "VERSION=",
            "ID=",
            "ID_LIKE=debian",
            "PRETTY_NAME=",
            "HOME_URL=",
            "SUPPORT_URL=",
            "BUG_REPORT_URL=",
            "LOGO=",
        ] {
            assert!(
                text.contains(required),
                "missing field {required} in os-release"
            );
        }
    }
}
