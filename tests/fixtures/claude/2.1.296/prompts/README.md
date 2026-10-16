# Claude Code prompt screens

This folder holds screens of **Claude Code 2.1.296**, captured with `tmux capture-pane -p` from a live session in a 100-column pane. The paths were replaced with an invented project. `agents::claude::ask` reads them:

- `permission.txt`: a Bash permission prompt. Pressing `1` approved it and `4` (its `No`) denied it.
- `question.txt`: one AskUserQuestion question with a single choice. Pressing `2` answered `Green` and submitted it.
- `questions.txt`: two questions, the first with several choices. The notch offers no buttons for it.
- `answered.txt`: the screen after the question was answered, with the input box back.
