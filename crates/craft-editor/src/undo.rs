use crate::state::EditorState;

type StateOp = Box<dyn Fn(&mut EditorState)>;

pub struct UndoRedo {
    actions: Vec<Action>,
    current: i32,
    max_steps: usize,
}

struct Action {
    #[allow(dead_code)]
    name: String,
    do_ops: Vec<StateOp>,
    undo_ops: Vec<StateOp>,
}

impl UndoRedo {
    pub fn new(max_steps: usize) -> Self {
        Self {
            actions: Vec::new(),
            current: -1,
            max_steps,
        }
    }

    pub fn begin_action(&mut self, name: &str) {
        while (self.current + 1) < self.actions.len() as i32 {
            self.actions.pop();
        }
        self.actions.push(Action {
            name: name.to_string(),
            do_ops: Vec::new(),
            undo_ops: Vec::new(),
        });
        self.current += 1;
    }

    pub fn add_do<F: Fn(&mut EditorState) + 'static>(&mut self, f: F) {
        if let Some(action) = self.actions.last_mut() {
            action.do_ops.push(Box::new(f));
        }
    }

    pub fn add_undo<F: Fn(&mut EditorState) + 'static>(&mut self, f: F) {
        if let Some(action) = self.actions.last_mut() {
            action.undo_ops.push(Box::new(f));
        }
    }

    pub fn commit_action(&mut self) {
        while self.actions.len() > self.max_steps {
            self.actions.remove(0);
            self.current -= 1;
        }
    }

    pub fn undo(&mut self, state: &mut EditorState) -> bool {
        if self.current < 0 {
            return false;
        }
        for undo_op in self.actions[self.current as usize].undo_ops.iter().rev() {
            undo_op(state);
        }
        self.current -= 1;
        true
    }

    pub fn redo(&mut self, state: &mut EditorState) -> bool {
        let next = self.current + 1;
        if next >= self.actions.len() as i32 {
            return false;
        }
        for do_op in &self.actions[next as usize].do_ops {
            do_op(state);
        }
        self.current = next;
        true
    }

    pub fn has_undo(&self) -> bool {
        self.current >= 0
    }

    pub fn has_redo(&self) -> bool {
        (self.current + 1) < self.actions.len() as i32
    }

    pub fn clear(&mut self) {
        self.actions.clear();
        self.current = -1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::EditorState;

    #[test]
    fn undo_redo_single_action() {
        let mut ur = UndoRedo::new(100);
        let mut state = EditorState::default();

        ur.begin_action("test");
        ur.add_do(|s| s.ui.status_message = "done".into());
        ur.add_undo(|s| s.ui.status_message = "undone".into());
        ur.commit_action();

        ur.undo(&mut state);
        assert_eq!(state.ui.status_message, "undone");

        ur.redo(&mut state);
        assert_eq!(state.ui.status_message, "done");
    }

    #[test]
    fn undo_returns_false_when_empty() {
        let mut ur = UndoRedo::new(100);
        let mut state = EditorState::default();
        assert!(!ur.undo(&mut state));
        assert!(!ur.redo(&mut state));
    }

    #[test]
    fn max_steps_trims_old_actions() {
        let mut ur = UndoRedo::new(2);
        for i in 0..5 {
            ur.begin_action(&format!("action-{i}"));
            ur.add_do(|_| {});
            ur.add_undo(|_| {});
            ur.commit_action();
        }

        assert_eq!(ur.actions.len(), 2);
        assert_eq!(ur.current, 1);
    }

    #[test]
    fn new_action_discards_redo_stack() {
        let mut ur = UndoRedo::new(100);
        let mut state = EditorState::default();

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
}
