# cornercase-notch

A plugin for [cornercase](https://github.com/usecornercase/cornercase-terminal) that shows the coding agents that need you at the top centre of your Mac's screen, hanging from the notch (or from the top edge on a Mac without one), and lets you answer Claude Code's permission prompts and questions there.

cornercase already tells you in its sidebar that an agent is waiting. This plugin is for when you are looking at something else: a small pill appears at the notch, and a click takes you to the pane.

It is made by jacogaz and is not part of cornercase.

## Install

```sh
cornercase plugin install jacogaz/cornercase-notch
cornercase plugin enable jacogaz.notch
```

The second step can also be done in Settings › Plugins. Nothing runs until you enable it, and a plugin runs as you: it asks for what it needs in its manifest (`cornercase plugin list`) and cornercase shows it before you turn it on.

## What it shows

Any agent cornercase knows (Claude Code, Codex, Gemini CLI, opencode, Grok Build, Cursor CLI, and any it learns later) gets a card when it:

- **waits for you** (a permission prompt, a question, or anything else);
- **failed**;
- **finished** and you were not looking.

Agents that are working, running a background shell, idle or pending get no card. Each card names the agent, the tab (or the project and workspace) and how long ago it started.

Most of the time you see a **pill**: Rinco, the plugin's mascot, and one line about the most urgent card, with `+2` when others wait. It opens into cards while the pointer is over it, or for a few seconds when something new needs you, and folds back four seconds after the pointer leaves. A finished agent only flashes the pill for a few seconds. Click a card, or its `open` button, to show that pane and bring your terminal forward.

## Answering

A card gets answer buttons only for agents whose screen the plugin can read reliably, checked against real screens kept as test fixtures:

| agent | buttons | why |
|-------|---------|-----|
| Claude Code | permission prompts: `allow`, `deny`; single questions: one per option | checked live; a digit picks the option and submits it |
| Codex | command approvals: `allow`, `deny` | checked live on 0.163.0; a digit selects and submits. Its questions, patch approvals and permission requests get `open` only |
| Gemini CLI, Grok Build | `open` only | could not be checked live |
| opencode | `open` only | allow needs Enter on a highlight that plain screen text does not show |
| Cursor CLI | `open` only | no digits, and its labels change between releases |

Pressing a button types that option's digit in the pane. Several questions, checkboxes, free text and anything else get `open` only.

The buttons are offered only when cornercase's screen of the pane still shows the same prompt right before the digit is typed. If the prompt is gone the plugin presses nothing and shows the pane instead. The command is shown on the card, wrapped on up to three lines, and `allow` is offered only when all of it is visible and the agent did not cut it short; otherwise there is `deny` and `open`, and you read the rest in the pane.

## Limits

- macOS only.
- Local windows only: it does not run under `cornercase remote`, which has no socket on the window's machine.
- "Looking" means your terminal app is the frontmost app. A card for the tab you are looking at is hidden, but the plugin cannot tell a different window or tab of the same terminal app from the one cornercase shows.
- One notch per Mac: a second window waits and takes over when the first closes.
- Only Claude Code's and Codex's prompts can be answered. Another agent gets a parser in `src/agents/<agent>/` and a line in `agents::answers` and `agents::ask_screen`.

## Development

```sh
scripts/dev.sh                 # builds and copies the binary to bin/
cornercase plugin link .       # then: cornercase plugin enable jacogaz.notch
cargo test
```

Without a window, `cornercase-notch board` prints the board as JSON and `cornercase-notch act '<action as JSON>'` runs one action, as a click would. Both use `$CORNERCASE_BIN` (else `cornercase` from the `PATH`). See [DESIGN.md](DESIGN.md) for how it works and for the release loop. A pull request that changes the app bumps `version` in `Cargo.toml` and adds its `CHANGELOG.md` section; it never touches the manifest's `version` or `[download.*]` tables, which only the release commit writes.

## License

MIT
