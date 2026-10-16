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

It reads no files of cornercase's and never sends anything but a digit. The events child lives in the plugin's process group, which cornercase's supervisor stops as a whole, so no extra termination handling was added for it.

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

"The same ask" means everything the user could have seen, not only what the card shows: an `Ask` carries a fingerprint of the whole prompt region as read (from the rule or title above it down to the footer: Codex's `Environment`, `Reason`, `Permission rule` and `Thread` lines, Claude Code's option descriptions and tip), and the re-check needs an equal one. The fingerprint is never shown. A prompt is also only recognised when its footer is the last non-empty line of the screen, so an older prompt left above newer content is not an ask.

A command keeps the screen's line boundaries and spacing (only the prompt's own indentation is removed); the card shows its lines, and the pill and outcome texts join them with `⏎` for display only.

The card says what an answer did (`✓ You picked …`, `◐ Running …`, `✕ Denied …`) only after the action reports that the digit was sent: the action thread passes its result to the main thread, which dismisses the card without a success text when the re-check refused or `keys` failed. Each button holds its own action and the render it belongs to, so a click on a shape from a redrawn panel does nothing.

Every agent cornercase reports gets answering, with one rule: where the plugin can read its screen strictly. Each parser (`src/agents/<agent>/ask.rs`) is anchored to the bottom of the screen (its footer or last row is the last non-empty line), reads one prompt, and returns nothing for anything it does not know; every `Ask` carries a fingerprint of the whole prompt region that the re-check compares. A `Choice` carries the key sequence to press, sent in one `cornercase keys` call; an answer names its choice by index. Never offered, whatever the agent: a free-text or "Other" row, a row that remembers a choice (always, don't ask again, allowlist, run everything), several questions, multiple selection.

| agent | permission | question | keys | grounded in |
|-------|------------|----------|------|-------------|
| Claude Code | `Yes` and the `No` row | numbered rows | digits | live 2.1.296, 2.1.302 |
| Codex | `Yes, proceed` / `Yes, just this once`, `No, and tell Codex…` | `request_user_input`, one question, not "None of the above" | digits | live 0.163.0 |
| opencode | `esc` to reject; never allow | `N. label` rows, not "Type your own answer" | `esc`; digits | live 1.19.3 |
| Gemini CLI | the digit of a first row that is exactly `Allow once`; the row ending `(esc)` | rows, not the last (custom) row; at most nine rows | digits | upstream snapshots of 0.64.0 |
| Grok Build | the digit of `Yes, proceed` or `Yes`; of `No, reject (type to add feedback)` | none (open only) | digits only (never Enter: its cursor may be on the always-approve row) | source; hand-made fixtures |
| Cursor CLI | `y` when a `Run (once) (y)`-style row exists; `n` when a row shows `(esc or n)` / `(n)` | one "Clarifying Questions" question | `up`×(rows−1), `down`×(N−1), `enter` (Up and Down are clamped there, so this is position-independent); never `j`/`k` | source; hand-made fixtures |

**opencode's status exception.** cornercase never reports opencode as `waiting`: during its permission prompt and its question the pane stays `working`. So the screens of opencode's working panes are read on every refresh (the worker keeps refreshing every second while one exists), and a card exists only when a prompt is recognised; it is never a bare "waiting" card. The exception is one function, `agents::polled`, next to `agents::answers`. opencode also prints a sidebar on the same lines as the prompt, so the main column is found from the sidebar's footer: the screen's last non-empty line is `  ┃`, spaces, then `• OpenCode <version>`, and the column of its bullet is the sidebar's left edge (165 in the live fixtures). It is chrome below the prompt, so prompt text cannot move it; nothing is inferred from runs of spaces. Everything at or right of the edge is ignored. Within the main column every line of the prompt or question must be whole: no text after a run of ten spaces, and no text in the last column before the edge, where the sidebar could have covered more. Anything but blank lines after the prompt in the main column (indented text, another prompt, a dialog) gives no ask, and so does a first non-blank line above it that has a bar. A screen without that footer (no sidebar) is read whole, and then any text after a run of ten spaces in a prompt line gives no ask.

**Grok Build's questions.** In its question view a digit is typed as text while the cursor is on the free-text row, and the cursor is a background colour: the `(●)` mark is the selection, not the cursor. Plain text cannot tell the two apart, and there is no live screen to learn from, so a question gets `open` only. Its permission prompts keep their digits: if feedback is being typed, the reject row's label stops being exactly `No, reject (type to add feedback)`, which the parser requires, so the prompt is not recognised.

**opencode and Enter.** Its permission prompt has no digits and no letters, only Left/Right (which wrap), Enter and Esc. Esc rejects from any highlight, so `deny` is `esc`. Enter picks the highlighted button, and a fresh prompt starts on `Allow once`, but the highlight is only a background colour, invisible in screen text, and a hover or an `h`/`l` typed meanwhile moves it; a click on `allow` could therefore reject. So there is no `allow` for opencode. It would need cornercase's `read` to expose the highlight (cell styles). The action still reads the screen again after a press: if it shows `△ Always allow` or `△ Reject permission`, it presses Esc once, shows the pane and reports the card as not answered.

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

Every change to the app (`src`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `cornercase-plugin.toml`) needs a new version in `Cargo.toml` and a `## <version>` section in `CHANGELOG.md`; CI checks it on pull requests (`.github/version.sh check`). A pull request bumps those two only: the manifest's own `version` and its `[download.*]` tables are written together by the release commit, so `main` never advertises an old binary under a new version (`check` fails a pull request that changes either, and a manifest whose `version` differs from the version in its own download URLs).

cornercase installs a prebuilt plugin from `[download.<target>]` in the manifest, with an https URL and the archive's sha256, so the manifest of the released commit must name its own archives; a moving "latest" asset cannot work. On each push to `main`, the release workflow runs `version.sh release`, which answers one of:

- `new`: there is no `v<version>` tag. It builds both macOS targets, packs each as `cornercase-notch-<target>.tar.gz` holding only `bin/cornercase-notch` (`.github/package.sh`, byte for byte reproducible: fixed times, numeric owner, no gzip name), sets the manifest's version and `[download.*]` tables (`.github/manifest.sh`), commits `chore: release v<version> [skip ci]`, pushes the tag first, creates the GitHub Release on it with the archives and the changelog section, and only then pushes the commit to `main` as a fast-forward (`.github/publish.sh new`).
- `resume`: the tag exists but the release is not whole: the release commit never reached `main`, or the GitHub release or one of its archives is missing. It checks the tag out, rebuilds from it, requires each archive's sha256 to equal the one the tagged manifest names (otherwise it fails and changes nothing), creates the release or uploads only the missing archives, and pushes the tagged commit to `main` if it is not there (a fast-forward; if `main` has moved past it, `version.sh` fails early and the tag and release have to be deleted or the version bumped). A tag alone never decides what reaches `main`: the commit must be a genuine release commit (`genuine_release`): its parent is on `main`, its author is `github-actions[bot]`, and it changes only `cornercase-plugin.toml`, and that file is byte for byte what `manifest.sh` writes from the parent's manifest, the version in the commit's own download URLs and the checksums it names (so nothing can be added before, between or after the tables). That version must also be the one in `Cargo.toml` at the commit, and the check runs in a subshell so it cannot change its caller's variables. The workflow checks out the verified commit's SHA, not a name rebuilt from a version. Protecting `v*` tags with a repository ruleset is still advised.
- `no`: the release is whole. If the app changed since the tag it fails with "bump the version".

So `main` never names an archive that does not exist, and an interrupted release is finished by the next run instead of demanding a new version. The archives are built from the commit before the release commit, which differs only in the manifest. The rebuilt binary must be identical to the first one for a resume to be accepted; on a changed toolchain image it may not be, which fails loudly. `.github/test.sh` tries all of this in scratch repositories with a bare repository as origin and a stub `gh`. For the release commit to reach `main`, branch protection must allow `github-actions[bot]` to push.
