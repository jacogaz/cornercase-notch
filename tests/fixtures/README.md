# Agent prompt screens

Screens of coding agents waiting for an answer, as `cornercase read --pane N` (or `tmux capture-pane -p`) printed them from a live session. The paths were replaced with invented ones. The parsers in `src/agents/<agent>/ask.rs` read them, and they are the only ground truth those parsers have: an agent gets answer buttons only when its screen is parsed reliably and tested against a real one.

- `claude/2.1.296/prompts/`: Claude Code 2.1.296 in a 100-column pane (live).
  - `permission.txt`: a Bash permission prompt. Pressing `1` approved it and `4` (its `No`) denied it.
  - `question.txt`: one AskUserQuestion question with a single choice. Pressing `2` answered `Green` and submitted it.
  - `questions.txt`: two questions, the first with several choices. The notch offers no buttons for it.
  - `answered.txt`: the screen after the question was answered, with the input box back.
- `claude/2.1.302/prompts/question.txt`: the same kind of question on Claude Code 2.1.302, the whole screen.
- `codex/0.163.0/prompts/permission.txt`: a Codex 0.163.0 command approval. Pressing a row's digit selects and submits it (`1` approved, `3` denied).
- `codex/0.163.0/prompts/question.txt`: a Codex `request_user_input` question (live). A digit selects and submits.
- `opencode/1.19.3/prompts/`: opencode 1.19.3 (live), with its sidebar on the prompt's lines: `permission.txt` (digits do nothing; Esc rejects; Enter acts on a highlight the screen text does not show, so it is never pressed), `always-confirm.txt` (the confirmation stage Enter on `Allow always` opens; Esc goes back) and `question.txt` (a digit selects and submits). opencode's status stays `working` during both prompts.
- `gemini/0.64.0/prompts/`: **transcribed from Gemini CLI's own test snapshots** (`ToolConfirmationQueue` and `AskUserDialog`), not captured from a live session, since it needs a login. The tool box is re-drawn at a fixed 80 columns.
- `grok/1.0.45/prompts/`: **made by hand** from Grok Build's source (`permission_view.rs`, `question_view.rs`), not captured; `question.txt` is kept as a screen that gets no ask, since it needs a login. The accent bar is assumed to be one character followed by two spaces.
- `cursor/2026.01.23/prompts/`: Cursor Agent 2026.01.23 (live; `cursor-agent` 2026.01.23-916f423).
  - `permission.txt`, `permission-2.txt`: a shell command prompt, a box at the bottom of the screen with a box above it holding `$  <command> in <dir>`. `y` runs the command once; `n` does not deny, it opens the reason box (Enter on an empty reason returns to the same prompt); `q` rejects outright. cornercase reports the pane as `idle` during the prompt.
  - `reject-reason.txt`: what `n` opens, a `Reason for rejection` box. It gets no ask.
  - `chat-question.txt`: the model wrote a question as chat, so it is not a question prompt and gets no ask.
  - `question.txt`: **made by hand** from the installed bundle's source (Cursor's question tool could not be triggered live).

Where a fixture is not live, the parser reads the wording the source says, and a newer release that words it differently is simply not recognised (the card gets `open`).
