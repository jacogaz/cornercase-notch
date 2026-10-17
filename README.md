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

Every supported agent gets the same answering, only where the plugin can read its screen strictly. Each parser is anchored to the bottom of the screen and tested against screens kept as fixtures; a prompt it does not recognise gets `open` only.

| agent | permission | single question | keys | grounded in |
|-------|------------|-----------------|------|-------------|
| Claude Code | `allow`, `deny` | one button per option | the option's digit | live screens (2.1.296, 2.1.302) |
| Codex | command approvals: `allow`, `deny` | one per option (not "None of the above") | the digit | live screens (0.163.0) |
| opencode | `deny` only (see below) | one per option (not "Type your own answer") | `esc`; the digit | live screens (1.19.3) |
| Gemini CLI | `allow` ("Allow once" only), `deny` | one per option (not the custom row) | the digit | its upstream snapshots (0.64.0), no live screen yet |
| Grok Build | `allow`, `deny` | `open` only (see below) | the digit | its source, hand-made fixtures, no live screen yet |
| Cursor CLI | `allow`, `deny` | one per option (one question only) | `y` / `q`; `up`…`down`…`enter` | permission: live, 2026.01.23; question: source, hand-made fixture |

opencode's permission prompt has no `allow`: its only way to allow is Enter on the highlighted button, and the highlight (a background colour) can be moved by the keyboard or by the mouse hovering over the panel without anything changing in the screen text, so Enter could reject. An `allow` would need cornercase's `read` to expose the highlight (cell styles). Until then the card offers `deny` (Esc, which rejects from any highlight) and `open`. Grok Build's questions get `open` only: a digit is typed as text while its cursor is on the free-text row, and plain text cannot show where the cursor is.

Never offered, for any agent: a free-text or "Other" row, a row that remembers a choice (always allow, don't ask again, an allowlist, run everything), several questions at once, multiple selection.

Pressing a button presses the keys in the table in the pane, in one `cornercase keys` call. Several questions, checkboxes, free text and anything else get `open` only.

The buttons are offered only when cornercase's screen of the pane still shows the same prompt right before the digit is typed. If the prompt is gone the plugin presses nothing and shows the pane instead. The command is shown on the card, wrapped on up to three lines, and `allow` is offered only when all of it is visible and the agent did not cut it short; otherwise there is `deny` and `open`, and you read the rest in the pane.

## Limits

- macOS only.
- Local windows only: it does not run under `cornercase remote`, which has no socket on the window's machine.
- "Looking" means your terminal app is the frontmost app. A card for the tab you are looking at is hidden, but the plugin cannot tell a different window or tab of the same terminal app from the one cornercase shows.
- One notch per Mac: a second window waits and takes over when the first closes.
- Gemini CLI, Grok Build and Cursor CLI's questions are read from their source and upstream snapshots, not from a live screen; their wording can differ in a newer release, in which case a prompt is simply not recognised and gets `open`. Another agent gets a parser in `src/agents/<agent>/` and a line in `agents::answers` and `agents::ask_screen`.

## Development

```sh
scripts/dev.sh                 # builds and copies the binary to bin/
cornercase plugin link .       # then: cornercase plugin enable jacogaz.notch
cargo test
```

Without a window, `cornercase-notch board` prints the board as JSON and `cornercase-notch act '<action as JSON>'` runs one action, as a click would. Both use `$CORNERCASE_BIN` (else `cornercase` from the `PATH`). See [DESIGN.md](DESIGN.md) for how it works and for the release loop. A pull request that changes the app bumps `version` in `Cargo.toml` and adds its `CHANGELOG.md` section; it never touches the manifest's `version` or `[download.*]` tables, which only the release commit writes.

## License

MIT
