//! Reading `ENVENB_*` settings, falling back to the `ENVFISH_*` names the app
//! used before the 2026-09 rename so existing shell profiles and CI configs
//! keep working.

/// Read `ENVENB_<suffix>`, falling back to `ENVFISH_<suffix>`.
pub fn var(suffix: &str) -> Option<String> {
    std::env::var(format!("ENVENB_{suffix}"))
        .or_else(|_| std::env::var(format!("ENVFISH_{suffix}")))
        .ok()
}

/// Whether `ENVENB_<suffix>` or `ENVFISH_<suffix>` is set at all.
pub fn is_set(suffix: &str) -> bool {
    std::env::var_os(format!("ENVENB_{suffix}")).is_some()
        || std::env::var_os(format!("ENVFISH_{suffix}")).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unset_suffix_reads_as_none() {
        assert!(var("DEFINITELY_NOT_SET_ANYWHERE").is_none());
        assert!(!is_set("DEFINITELY_NOT_SET_ANYWHERE"));
    }
}
