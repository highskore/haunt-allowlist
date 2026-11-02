//! Offers an exact `Bash(<command>)` allow rule once you approved the same command in the same
//! project 3 times in Haunt. Haunt writes the rule only when you accept the suggestion there.
// wit-bindgen's flattened `notify` import has 14 params.
#![allow(clippy::too_many_arguments)]

wit_bindgen::generate!({ path: "wit/haunt-plugin.wit", world: "plugin" });

use std::collections::BTreeSet;

use haunt::plugin::allowlist::{self, Approval};
use haunt::plugin::kv;
use haunt::plugin::log::log;
use haunt::plugin::types::LogLevel;

pub const SUGGEST_AFTER: u32 = 3;
const KEY: &str = "proposed";

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

/// The core's `allow_done` is in-memory, so a dismissed suggestion would come back after every
/// restart; the proposed set therefore lives in kv.
fn proposed() -> BTreeSet<(String, String)> {
    kv::get(KEY).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default()
}

fn save(proposed: &BTreeSet<(String, String)>) {
    match serde_json::to_string(proposed) {
        Ok(j) => {
            if let Err(e) = kv::set(KEY, &j) {
                log(LogLevel::Warn, &format!("allowlist: proposed set not saved: {e}"));
            }
        }
        Err(_) => log(LogLevel::Warn, "allowlist: proposed set not serializable"),
    }
}

fn check() {
    let mut proposed = proposed();
    let fresh = due(allowlist::approvals(SUGGEST_AFTER), &proposed);
    if fresh.is_empty() {
        return;
    }
    for (project, command) in fresh {
        // A refusal (say, a project that is a symlink) won't change by asking again.
        if let Err(e) = allowlist::propose(&project, &command) {
            log(LogLevel::Info, &format!("not proposing {command:?} in {project}: {e}"));
        }
        proposed.insert((project, command));
    }
    save(&proposed);
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

    fn row_menu(_surface: String) -> Vec<MenuItem> {
        Vec::new()
    }

    fn on_row_moved(_surface: String, _group: String, _before: Option<String>) {}

    fn on_theme_changed() {}
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
    fn saved_set_still_skips_after_a_restart() {
        let mut seen = BTreeSet::new();
        seen.insert(("/p".to_owned(), "cargo test".to_owned()));
        seen.insert(("/q".to_owned(), "echo \"hi there\"".to_owned()));
        let restored: BTreeSet<(String, String)> = serde_json::from_str(&serde_json::to_string(&seen).unwrap()).unwrap();
        assert_eq!(restored, seen);
        assert!(due(vec![approval("/p", "cargo test", 5), approval("/q", "echo \"hi there\"", 9)], &restored).is_empty());
        assert_eq!(due(vec![approval("/r", "ls", 3)], &restored).len(), 1, "an unrecorded rule is still proposed");
    }

    #[test]
    fn unreadable_saved_set_starts_empty() {
        let none: BTreeSet<(String, String)> = serde_json::from_str("not json").unwrap_or_default();
        assert!(none.is_empty());
        assert_eq!(due(vec![approval("/p", "cargo test", 3)], &none).len(), 1);
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
