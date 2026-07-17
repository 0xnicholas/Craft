use craft_editor::state::EditorState;
use craft_editor::undo::UndoRedo;

#[test]
fn undo_stack_push_and_undo() {
    let mut state = EditorState::default();
    let mut ur = UndoRedo::new(100);
    ur.begin_action("test status");
    ur.add_do(|s| s.ui.status_message = "done".into());
    ur.add_undo(|s| s.ui.status_message = "undone".into());
    ur.commit_action();

    assert_eq!(state.ui.status_message, "");
    assert!(ur.undo(&mut state));
    assert_eq!(state.ui.status_message, "undone");
}

#[test]
fn redo_after_undo_restores_state() {
    let mut state = EditorState::default();
    let mut ur = UndoRedo::new(100);
    ur.begin_action("test");
    ur.add_do(|s| s.ui.status_message = "final".into());
    ur.add_undo(|s| s.ui.status_message = "initial".into());
    ur.commit_action();

    ur.undo(&mut state);
    assert_eq!(state.ui.status_message, "initial");
    ur.redo(&mut state);
    assert_eq!(state.ui.status_message, "final");
}

#[test]
fn multiple_undo_actions() {
    let mut state = EditorState::default();
    let mut ur = UndoRedo::new(100);

    ur.begin_action("first");
    ur.add_do(|s| s.ui.status_message = "1".into());
    ur.add_undo(|s| s.ui.status_message = "0".into());
    ur.commit_action();

    ur.begin_action("second");
    ur.add_do(|s| s.ui.status_message = "2".into());
    ur.add_undo(|s| s.ui.status_message = "1".into());
    ur.commit_action();

    assert!(ur.undo(&mut state));
    assert_eq!(state.ui.status_message, "1");
    assert!(ur.undo(&mut state));
    assert_eq!(state.ui.status_message, "0");
    assert!(!ur.undo(&mut state));
}

#[test]
fn new_action_discards_redo_stack() {
    let mut state = EditorState::default();
    let mut ur = UndoRedo::new(100);

    ur.begin_action("first");
    ur.add_do(|_| {});
    ur.add_undo(|_| {});
    ur.commit_action();

    ur.undo(&mut state);

    ur.begin_action("second");
    ur.add_do(|_| {});
    ur.add_undo(|_| {});
    ur.commit_action();

    assert!(!ur.has_redo());
}

#[test]
fn max_steps_trims_oldest() {
    let _state = EditorState::default();
    let mut ur = UndoRedo::new(3);

    for i in 0..5 {
        ur.begin_action(&format!("{i}"));
        ur.add_do(move |s| s.ui.status_message = format!("do-{i}"));
        ur.add_undo(move |s| s.ui.status_message = format!("undo-{i}"));
        ur.commit_action();
    }

    assert_eq!(ur.actions_len(), 3);
}

#[test]
fn undo_empty_returns_false() {
    let mut state = EditorState::default();
    let mut ur = UndoRedo::new(100);
    assert!(!ur.undo(&mut state));
    assert!(!ur.has_undo());
    assert!(!ur.has_redo());
}

#[test]
fn clear_removes_all_actions() {
    let _state = EditorState::default();
    let mut ur = UndoRedo::new(100);
    ur.begin_action("a");
    ur.add_do(|_| {});
    ur.add_undo(|_| {});
    ur.commit_action();
    ur.clear();
    assert!(!ur.has_undo());
    assert!(!ur.has_redo());
}

#[test]
fn undo_redo_with_editorstate_does_not_crash() {
    let mut state = EditorState::default();
    let mut ur = UndoRedo::new(100);
    ur.begin_action("noop");
    ur.add_do(|_| {});
    ur.add_undo(|_| {});
    ur.commit_action();

    assert!(ur.undo(&mut state));
    assert!(ur.redo(&mut state));
}
