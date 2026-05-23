use std::path::PathBuf;

/// Resolve the SQLite DB path. Honors `DINHEIROS_DB_PATH` if set; otherwise
/// returns `<data_local_dir>/dinheiros/data.db`. Used by the UI and by the
/// seed binary so both share one source of truth.
pub fn default_db_path() -> PathBuf {
    if let Ok(override_path) = std::env::var("DINHEIROS_DB_PATH") {
        return PathBuf::from(override_path);
    }
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dinheiros")
        .join("data.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_var_override_wins() {
        // Safety: tests in this module run serially via the lock below.
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("DINHEIROS_DB_PATH", "/tmp/test-override.db");
        assert_eq!(default_db_path(), PathBuf::from("/tmp/test-override.db"));
        std::env::remove_var("DINHEIROS_DB_PATH");
    }

    #[test]
    fn default_when_env_unset() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("DINHEIROS_DB_PATH");
        let p = default_db_path();
        assert!(p.ends_with("dinheiros/data.db"), "got {:?}", p);
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
