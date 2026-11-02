# haunt-allowlist

Allow-rule suggestions for Claude Code as a first-party Haunt plugin (`dev.haunt.allowlist`). Once you have approved the same Bash command in the same project 3 times by clicking Allow in Haunt, it proposes the exact rule `Bash(<command>)` for that project's `.claude/settings.local.json`.

The plugin only proposes. Haunt shows the proposal as a suggestion, and writes the rule only when you accept it there. The plugin can't write any file. When you accept, Haunt checks everything again:

- the command was approved in Haunt (not in the terminal, not by a plugin) by Claude Code
- the command it saw is exactly the one shown, with no hidden characters
- there are no wildcards (`*`, `(`, `)`), only printable ASCII, and at most 1024 bytes
- the project is a canonical, non-symlinked folder you own
- the file goes through Haunt's symlink-safe write

Haunt's core counts the approvals. This plugin decides when to offer a rule: after 3, once per rule per run.

## Capabilities

| capability | why |
|---|---|
| `events = ["agent.tool"]` | an approval is followed by the tool running, so each tool event is when it checks the counts |
| `allowlist_suggest` | `allowlist.approvals` (counts per project and exact command) and `allowlist.propose` (offer a rule; never writes) |

## Build, test, install

```sh
cargo test                                     # native unit tests
cargo build --release --target wasm32-wasip2   # or ./build.sh
haunt plugin install .
```

It needs Haunt with `haunt:plugin@0.2` (`api = "0.2"`). `wit/haunt-plugin.wit` is a copy of Haunt's, pinned at that version.

## Licence

MIT, see `LICENSE`.
