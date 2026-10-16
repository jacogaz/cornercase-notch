# Design

Why this plugin works the way it does. It is a plugin for cornercase written in Rust, by jacogaz; it is not part of cornercase and uses only what a plugin can: cornercase's command line.

## One window process

The manifest declares one `[[window]]` entry, `panel`. cornercase runs it next to each local window, with that window's environment, and restarts it if it ends. A window process, not a service, because the notch is the window's screen and its idea of "looking" belongs to the machine the window is on; a service in the server could not know which app is in front. The price is that two local windows run two processes, which the lock below turns into one panel. It cannot run under `cornercase remote`: there the window's machine has no socket.

One process does everything: it follows events, builds the board, draws it and acts on clicks. There is no pipe between a client and a child.

## The calls it makes

All through `$CORNERCASE_BIN` (else `cornercase` from the `PATH`), as an argv, never a shell:

- `status --panes --json`: every pane with its agent, status, tab, names and whether it is shown. Every field is optional and unknown ones are ignored, so a newer cornercase does not break it.
- `read --pane N`: the screen of a pane. Only for waiting panes of an agent that can be answered.
- `events --json`: kept running; each line only means "something changed", plus the pane and the old and new status when it is a status event.
- `keys --pane N <digit>` and `focus --pane N`: only on a click.

It reads no files of cornercase's and never sends anything but a digit.

## The board

`board::build` turns the panes into cards. It is pure: the panes, what the process has seen of the events and the clock go in, and a list of cards comes out; a screen reader function is called only for waiting panes of agents that `answers`. Any pane with an agent can get a card:

- waiting becomes a card with `open` only, or with buttons when the screen is a prompt that agent's parser recognises (`agents::answers` and `agents::ask_screen` are the one switch, and a parser goes in `src/agents/<agent>/`);
- failed becomes a card with `open` only, in the error mood;
- done becomes a card with `open` only;
- working, a background shell, idle and pending get none.

Cards are ordered asks, waiting, failed, done, newest first. Each keeps its tab, and the board keeps which tab is shown. A card has the agent's display name, the tab's name unless that is only the program's, and the place (`project › workspace`). `since` is when this process saw the status begin, from the events; a status it did not see begin has none, and no age is shown.

The worker rebuilds at start, on each batch of events, and every second while a card waits (a screen can change while its agent waits) or a flash is live. It hands the latest board to the panel, which takes it on its 100 ms timer.

## The done flash

cornercase counts the shown tab as seen even while its window is in the background, so an agent in it goes from working to idle and never reports done. When a status event says a shown agent's pane went from working or a background shell to done or idle while nobody is looking, the pane gets a done card until it works or waits again, or for ten seconds. Agents in tabs out of sight report done by themselves and need nothing.

## Looking

The panel hides the cards of the shown tab while you look at the window. It cannot ask cornercase whether the window has the focus, so "looking" is an approximation: at start it walks up from its parent to the first process macOS knows as a regular app (the terminal), and it is looking while that app is the frontmost one. The walk reads parents with `PROC_PIDT_SHORTBSDINFO`, since the full process info is refused for another user's process, and a terminal starts its shell through the root-owned `login`. Under tmux the chain ends at tmux's server and nothing is found, so it is never looking. The panel shares the answer with the worker for the flash.

## Answering and its check

A click on a button runs an action on its own thread. Right before typing, the action reads the pane again: the digit is typed only when the agent is still one that `answers`, still waiting, the screen gives the same ask as the card showed and the key is one of its choices. Otherwise nothing is pressed and the pane is shown instead.

Which agents get buttons is a rule: only where the screen parse is reliable and tested against real screens kept as fixtures.

| agent | buttons | why |
|-------|---------|-----|
| Claude Code | permission prompts and single questions | checked live on 2.1.296 and 2.1.302; a digit picks the option and submits it |
| Codex | command approvals | checked live on 0.163.0; a digit selects and submits. Its `request_user_input` questions, patch approvals and permission requests are not recognised |
| Gemini CLI, Grok Build | none | not checked live (not signed in, not installed) |
| opencode | none | allow needs Enter on a highlight that is invisible in screen text |
| Cursor CLI | none | no digits, and its labels drift between releases |

The parsers are strict and anchored. Claude Code's: a permission prompt is the last ` Esc to cancel` line with numbered options under a ` Do you want to …?` line, `1. Yes` first and a `No` among them, whose number is read, never assumed; a question is the last `Enter to select · ↑/↓ to navigate · Esc to cancel` line with one tab, its question and numbered options ending in `Type something.`. Several questions, checkboxes, free text and anything else are not recognised.

Codex's command approval is the title line `  Would you like to run the following command?`, optional `Environment:`, `Reason:`, `Permission rule:` and `Thread:` lines, the command as `  $ …` lines (continuations indented), numbered rows `› N. label (key)` consecutive from 1 with at most one cursor, and as the last non-empty line `  Press enter to confirm or esc to cancel` (with `or o to open thread` allowed). Allow is the row starting `Yes, proceed` or `Yes, just this once`, deny the row starting `No, and tell Codex what to do differently`; both must exist, and a "don't ask again" row is never offered. Anything unknown above the command, a different title or footer, a skipped number or a second cursor gives no ask. When the agent elides the command (`[… N lines]`), only `deny` is offered. The shared reading of numbered rows is `agents/numbered.rs`.

A digit does not depend on where the cursor is, which arrow keys and Enter would.

Two limits, stated plainly. The check and the key press are two commands, so the prompt can change in between; the window is milliseconds and the screen is compared as a whole prompt, but cornercase has no "press this only if the screen is still that" call, which would close it. And the card must not let you allow what you cannot read: the command is shown wrapped in monospace on up to three lines, and `allow` is offered only when the whole command fits in them (measured when the card is laid out, with a character cap as a second guard), the agent did not elide it, and there is a command at all. Otherwise there is `deny` and `open`. A denial's text names the agent it was given to.

## One panel per Mac

The panel takes an exclusive `flock` on `cornercase-notch.lock` in `$TMPDIR` (per user on macOS). While another process holds it, the process shows nothing and tries again every two seconds, so a second window takes over when the first closes, and a killed process frees the lock by dying.

## The panel

A borderless, non-activating `NSPanel` at the status window level, on every space and over full-screen apps, drawn with `objc2` in the same binary; there is no Swift app to sign. It hangs from the notch (its width from the auxiliary areas, its height from the safe area; a 120 pt line on a Mac without one) and grows out of it and shrinks back with a damped spring (response 0.28 s, under 0.5 % overshoot), or jumps with Reduce Motion on. Content is laid out at its final size and clipped by the panel.

It is a pill (24 pt: Rinco, one line, `+n`) that expands into at most three cards while the pointer is over it or for four seconds after something new that needs an answer arrives. A finished agent shows the pill for 3.5 s and is dismissed, unless the panel is expanded. After an answer the card says what it did for 1.2 s (`✓ You picked Green`, `◐ Running …`, `✕ Denied`). It stays out of the way of the terminal: only waiting cards stay until answered.

Colours: needs `#f5d76e`, question `#b294ff`, done `#58e6a0`, working `#45d9e8`, failed and deny `#ff8aa3`; the card is `#1c1b20` on black. The system font, with its monospaced one for a running command.

## Rinco

Rinco is this plugin's mascot: a 16×16 pixel sprite in a hood with a screen for a face and an antenna whose light shows the state. It is on the pill and on every card, whatever the agent. Only what means something moves, urgent states fast and calm ones slowly. The frames are in `src/notch/rinco.json` and the timings are pinned by tests:

| mood | light | what it does | frames |
|------|-------|--------------|--------|
| working | `#45d9e8` | reads code while output scrolls | 4 × 240 ms, loops |
| needs | `#f5d76e` | hops and waves | 170, 170, 170, 450 ms, loops |
| question | `#c9b8ff` | looks up, thinking | 900, 650, 400, 650 ms, loops |
| done | `#58e6a0` | a small jump, then rests | 110, 150, 110, 150, 2200 ms, once |
| error | `#ff6b8b` | shakes | 4 × 60 ms then 1800 ms, once |

A permission card is needs, a question card is question, a failed card is error, a done card is done; after a deny the card is error, after an allow working, after picking an option done. Each sprite remembers when its pane entered its mood, so rebuilding the panel does not restart it but a new mood does. Anthropic's own marks are not used.

## Releases

Every change to the app (`src`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `cornercase-plugin.toml`) needs a new version in `Cargo.toml` and in the manifest, and a `## <version>` section in `CHANGELOG.md`; CI checks it on pull requests (`.github/version.sh check`).

cornercase installs a prebuilt plugin from `[download.<target>]` in the manifest, with an https URL and the archive's sha256, so the manifest of the released commit must name its own archives; a moving "latest" asset cannot work. On each push to `main`, the release workflow:

1. runs `version.sh release`: if `v<version>` exists it stops (and fails if the app changed since, which means the version was not bumped);
2. builds both macOS targets and packs each as `cornercase-notch-<target>.tar.gz` holding only `bin/cornercase-notch`;
3. rewrites the `[download.*]` tables at the end of the manifest (`.github/manifest.sh`) and commits `chore: release v<version> [skip ci]`;
4. pushes the tag first, creates the GitHub Release on it with the archives and the changelog section, and only then pushes the commit to `main` as a fast-forward.

So `main` never names an archive that does not exist. The archives are built from the commit before the release commit, which differs only in the manifest. If `main` moved meanwhile the last push fails; the tag stays, and the next change needs a version bump. `.github/test.sh` tries the two scripts on copies. For the release commit to reach `main`, branch protection must allow `github-actions[bot]` to push.
