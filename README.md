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

For **Claude Code** a permission prompt gets `allow` and `deny` buttons and a single question gets one button per option; pressing one types that option's digit in the pane. Everything else (several questions, checkboxes, free text, any other agent) gets `open` only.

The buttons are offered only when cornercase's screen of the pane still shows the same prompt right before the digit is typed. If the prompt is gone the plugin presses nothing and shows the pane instead. `allow` is offered only when the whole command fits on the card; for a longer one, open the pane and read it there.

## Limits

- macOS only.
- Local windows only: it does not run under `cornercase remote`, which has no socket on the window's machine.
- "Looking" means your terminal app is the frontmost app. A card for the tab you are looking at is hidden, but the plugin cannot tell a different window or tab of the same terminal app from the one cornercase shows.
- One notch per Mac: a second window waits and takes over when the first closes.
- Only Claude Code's prompts can be answered. Other agents can get a parser in `src/agents/<agent>/` and be listed in `board::answers`.

## Development

```sh
scripts/dev.sh                 # builds and copies the binary to bin/
cornercase plugin link .       # then: cornercase plugin enable jacogaz.notch
cargo test
```

Without a window, `cornercase-notch board` prints the board as JSON and `cornercase-notch act '<action as JSON>'` runs one action, as a click would. Both use `$CORNERCASE_BIN` (else `cornercase` from the `PATH`). See [DESIGN.md](DESIGN.md) for how it works and for the release loop.

## License

MIT
