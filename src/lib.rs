//! Offers an exact `Bash(<command>)` allow rule once you approved the same command in the same
//! project 3 times in Haunt. Haunt writes the rule only when you accept the suggestion there.
// wit-bindgen's flattened `notify` import has 14 params.
#![allow(clippy::too_many_arguments)]

wit_bindgen::generate!({ path: "wit/haunt-plugin.wit", world: "plugin" });

use std::collections::BTreeSet;
use std::sync::Mutex;

use haunt::plugin::allowlist::{self, Approval};
use haunt::plugin::log::log;
use haunt::plugin::types::LogLevel;

pub const SUGGEST_AFTER: u32 = 3;

/// (project, command) pairs already proposed in this run; the host forgets proposals on restart too.
static PROPOSED: Mutex<BTreeSet<(String, String)>> = Mutex::new(BTreeSet::new());

struct Plugin;

/// Approvals at the threshold that were not proposed yet.
pub fn due(approvals: Vec<Approval>, proposed: &BTreeSet<(String, String)>) -> Vec<(String, String)> {
    approvals
        .into_iter()
        .filter(|a| a.count >= SUGGEST_AFTER)
        .map(|a| (a.project, a.command))
        .filter(|k| !proposed.contains(k))
        .collect()
}

fn check() {
    let mut proposed = PROPOSED.lock().unwrap_or_else(|e| e.into_inner());
    for (project, command) in due(allowlist::approvals(SUGGEST_AFTER), &proposed) {
        // A refusal (say, a project that is a symlink) won't change by asking again.
        if let Err(e) = allowlist::propose(&project, &command) {
            log(LogLevel::Info, &format!("not proposing {command:?} in {project}: {e}"));
        }
        proposed.insert((project, command));
    }
}

impl Guest for Plugin {
    fn init(_settings: String) {
        check();
    }

    fn on_settings_changed(_settings: String) {}

    /// An approval is followed by the tool running, so each tool event is a moment to look.
    fn on_event(_event: AgentEvent) -> Option<Decoration> {
        check();
        None
    }

    fn palette_items(_mode: String, _query: String) -> Vec<PaletteItem> {
        Vec::new()
    }

    fn on_action(_action_id: String, _payload: String) {}
}

export!(Plugin);

#[cfg(test)]
mod tests {
    use super::*;

    fn approval(project: &str, command: &str, count: u32) -> Approval {
        Approval { project: project.into(), command: command.into(), count, last_at_ms: 0 }
    }

    #[test]
    fn proposes_after_three_approvals_only() {
        let none = BTreeSet::new();
        assert!(due(vec![approval("/p", "cargo test", 2)], &none).is_empty());
        assert_eq!(due(vec![approval("/p", "cargo test", 3), approval("/q", "ls", 1)], &none), [("/p".into(), "cargo test".into())]);
        assert_eq!(due(vec![approval("/p", "cargo test", 7)], &none).len(), 1);
    }

    #[test]
    fn proposes_each_rule_once() {
        let mut seen = BTreeSet::new();
        seen.insert(("/p".to_owned(), "cargo test".to_owned()));
        assert!(due(vec![approval("/p", "cargo test", 4)], &seen).is_empty());
        assert_eq!(due(vec![approval("/q", "cargo test", 3)], &seen).len(), 1, "another project is another rule");
    }

    #[test]
    fn manifest_asks_for_tool_events_and_suggestions_only() {
        let m = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/haunt-plugin.toml")).unwrap();
        assert!(m.contains("events = [\"agent.tool\"]") && m.contains("allowlist_suggest = true"), "{m}");
        for cap in ["pty_write", "notifications", "fs_read", "fs_write", "decisions_read"] {
            assert!(!m.contains(cap), "{cap}");
        }
    }
}
