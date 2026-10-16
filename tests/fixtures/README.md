# Agent prompt screens

Screens of coding agents waiting for an answer, as `cornercase read --pane N` (or `tmux capture-pane -p`) printed them from a live session. The paths were replaced with invented ones. The parsers in `src/agents/<agent>/ask.rs` read them, and they are the only ground truth those parsers have: an agent gets answer buttons only when its screen is parsed reliably and tested against a real one.

- `claude/2.1.296/prompts/`: Claude Code 2.1.296 in a 100-column pane.
  - `permission.txt`: a Bash permission prompt. Pressing `1` approved it and `4` (its `No`) denied it.
  - `question.txt`: one AskUserQuestion question with a single choice. Pressing `2` answered `Green` and submitted it.
  - `questions.txt`: two questions, the first with several choices. The notch offers no buttons for it.
  - `answered.txt`: the screen after the question was answered, with the input box back.
- `claude/2.1.302/prompts/question.txt`: the same kind of question on Claude Code 2.1.302, the whole screen.
- `codex/0.163.0/prompts/permission.txt`: a Codex 0.163.0 command approval. Pressing a row's digit selects and submits it (`1` approved, `3` denied).
