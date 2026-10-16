# Changelog

## 0.1.0

- First release. Coding agents that wait for you, failed or finished appear in a pill at the macOS notch, which opens into cards. Click one to show its pane and bring your terminal forward.
- Every supported agent (Claude Code, Codex, opencode, Gemini CLI, Grok Build, Cursor CLI) gets answer buttons for its permission prompts and single-choice questions, except opencode permissions (`deny` only, since Enter acts on an invisible highlight) and Grok Build questions (open only, since a digit may be typed as text). Multi-question, multi-select and free-text prompts, and any row that remembers a choice (always allow, don't ask again), are never offered; those can be opened. A command is shown in full (up to three lines) and `allow` is offered only when all of it is visible.
- Works with every agent cornercase reports, not only Claude Code.
