use craft_editor::io::recent::{self, RecentProjects};
use craft_editor::persist;

#[test]
fn dock_roundtrip() {
    let tmp = tempfile::TempDir::new().unwrap();
    temp_env::with_var("HOME", Some(tmp.path()), || {
        persist::save_dock(&persist::build_default_dock()).expect("save default dock");
        let loaded = persist::load_dock().expect("load default dock");
        assert!(
            loaded.main_surface().num_tabs() >= 2,
            "default dock has at least the documented tabs"
        );
    });
}

#[test]
fn dock_load_returns_none_when_missing() {
    let tmp = tempfile::TempDir::new().unwrap();
    temp_env::with_var("HOME", Some(tmp.path()), || {
        assert!(persist::load_dock().is_none());
    });
}

#[test]
fn recent_entries_cap_at_ten_welcome_takes_top_five() {
    let tmp = tempfile::TempDir::new().unwrap();
    temp_env::with_var("HOME", Some(tmp.path()), || {
        let mut r = RecentProjects::default();
        for i in 0..20 {
            let p = tmp.path().join(format!("proj{i}"));
            std::fs::create_dir_all(&p).unwrap();
            r.add_or_bump(&p);
        }
        let _ = recent::save(&r);
        let loaded = recent::load();
        assert_eq!(loaded.entries.len(), 10, "RecentProjects caps at 10");
        let top5: Vec<_> = loaded.entries.iter().take(5).collect();
        assert_eq!(top5.len(), 5);
        let first_name = top5[0]
            .root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        assert!(
            first_name.ends_with("proj19"),
            "newest entry first, got {first_name}"
        );
        let last_name = top5[4]
            .root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        assert!(
            last_name.ends_with("proj15"),
            "next-four newest follow, got {last_name}"
        );
    });
}

#[test]
fn add_or_bump_dedupes_and_promotes_to_top() {
    let tmp = tempfile::TempDir::new().unwrap();
    temp_env::with_var("HOME", Some(tmp.path()), || {
        let mut r = RecentProjects::default();
        let a = tmp.path().join("projA");
        let b = tmp.path().join("projB");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        r.add_or_bump(&a);
        r.add_or_bump(&b);
        r.add_or_bump(&a);
        assert_eq!(
            r.entries.len(),
            2,
            "re-opening same project does not duplicate"
        );
        assert_eq!(r.entries[0].root, a, "re-bumped entry moves to top");
        assert_eq!(r.entries[1].root, b);
    });
}
