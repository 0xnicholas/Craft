use craft_editor::watcher::Watcher;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn debounces_three_rapid_writes_into_one_event() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("scene.json");
    std::fs::write(&target, "{}").unwrap();

    let watcher = Watcher::new(dir.path()).expect("create watcher");
    std::thread::sleep(Duration::from_millis(50));

    for i in 0..3 {
        std::fs::write(&target, format!("{{\"i\":{i}}}")).unwrap();
        std::thread::sleep(Duration::from_millis(20));
    }

    let events = watcher.drain_debounced();
    let target_canon = target.canonicalize().unwrap();
    let scene_events: Vec<_> = events
        .into_iter()
        .filter(|e| {
            matches!(
                e,
                craft_editor::watcher::WatcherEvent::Changed(p)
                    if p.canonicalize().ok().as_ref() == Some(&target_canon)
            )
        })
        .collect();
    assert_eq!(
        scene_events.len(),
        1,
        "debounce should coalesce rapid writes"
    );
}
