# Playing a tutorial unattended

A tutorial author runs `tutorial-stepper play --auto` on a tutorial directory and names a transcript file. The stepper starts its own tmux server, one shell per window the index declares, types each command into that window's shell, and waits until that shell's prompt says the command ended. The shell is zsh when the index says `shell = "zsh"`, and bash otherwise. The transcript records every command that ran. When a command exits with a status the block did not declare, the run stops and names that command.

## Who it is for

The author, who wants the tutorial to run with nobody at the keyboard, whether the learners' shell is bash or zsh. Continuous integration runs the same command on the examples in this repository, under both shells. A learner's interactive session is not this command.

```mermaid
flowchart TD
  directory["Tutorial directory"]
  refuse["Refused before a shell starts"]
  server["Private tmux server and one bash or zsh per window"]
  command["One command, typed as written"]
  hook["Prompt hook reports the end and the status"]
  transcript["Transcript entry"]
  stop["Stop, name the command, exit non-zero"]
  directory --> refuse
  directory --> server
  server --> command
  command --> hook
  hook --> transcript
  hook --> stop
```

A format problem, a set-up script the author did not ask to run, or a command marked with a setting is refused before a shell starts. So is a program named in `needs` when it is missing, older than the declared minimum, or prints no version. The transcript file is not created. Anything else reaches the shell.

## What runs

```bash
tutorial-stepper play --auto examples/one-window --transcript transcript.toml
```

`--auto` and `--transcript` are both required. A sound run prints `1 command played` or `N commands played` on standard output, leaves standard error empty, and exits 0.

The stepper looks up `tmux` on `PATH`, then the shell the index declares: `zsh` when the index says so, and `bash` otherwise. The first one missing is named, and nothing is started. A zsh tutorial that runs its set-up script also needs `bash`, because that script is bash. The server uses its own socket and does not read a tmux configuration file. Each declared window is one shell of that kind, started in that window's folder under the run folder. Without `--run` the run folder is temporary and is removed when the run ends. `--run DIR` keeps that directory: a missing `DIR` is created, an empty directory is used, and a directory that already holds anything is refused without being changed. A `DIR` that is a file is refused.

When the index names a set-up script, it runs only with `--run-setup`. The script is `bash` on that file, in the run folder, after each window folder exists and before any page command. It runs as the user who started `play`. The stepper does not raise privileges and does not invoke `sudo` or `su`. On success the script's own output is discarded, and the confirmation stays the count of page commands. A script that exits with another status stops the run: the message names the script and that status, and no page command runs. A script path that is not a file is named, and nothing is typed. Without `--run-setup` the run stops before any command and names the script. `--run-setup` on a tutorial that names no script changes nothing. When the effective user is root, that script is refused before the needs check, the run folder, and the script itself. The message names the script and says it would run as root. The needs check and the page commands may run as root.

Before that script, and before the run folder is created, every `needs` entry is checked. Windows are checked in index order, and each window's entries in the order written. The first failure is the only one reported. A program that is not on `PATH` stops the run: the message names it and says it was not found. A minimum version runs `PROGRAM --version` and reads standard output and standard error as one text. The version is the first dotted number in that text, digits with at least one further group of `.digits`, matching `[0-9]+(\.[0-9]+)+`. `GNU bash, version 5.2.15(1)-release` yields `5.2.15`. The declared minimum is read the same way. The two numbers are compared component by component, as integers, and a missing component is 0, so `1.2` equals `1.2.0` and `1.10` is newer than `1.9`. An older program stops the run. The message names the program, the dotted number that was found, and the version the index requires. When the text contains no dotted number, the message names the program and says it printed no version. A need with no minimum checks that the program is on `PATH` and does not run it.

The shell loads the stepper's prompt hook. Bash reports from `PROMPT_COMMAND`. zsh reports from `precmd`. Either way the hook appends the exit status of the command that just ended, before the prompt is drawn. A command of several lines is sent as one bracketed paste, then Enter, so the shell runs it as one command. The stepper does not decide that a command ended by a deadline or by reading the output: a command that sleeps is waited out. The server is gone when the run ends, whether the run passed or failed.

Commands run in index page order, then fence order. A command marked `window=NAME` runs in that window. A command that names no window runs in the window of the previous command, and the first such command runs in the first window the index declares. A block marked `mode=show` or `mode=observe` is not run and does not change that window. A directory change stays in the shell that ran it. A fence body is one command, including a command continued over several lines. A fence with no braces is not run and does not appear in the transcript. An absent `status` expects 0. The first command whose reported status differs is the last one recorded. Later commands are not run. Standard output is empty. Standard error names the page, the line, the command, the status the shell reported, and the status the block declared, as `FILE:LINE: message`. The command exits 1.

A tutorial that `tutorial-stepper check` rejects prints the same problems and creates no transcript. The same is true of a tutorial this command refuses before starting a shell:

- The tutorial declares a set-up script and `--run-setup` was not given. The message names the script. The script is not run.
- The tutorial declares a set-up script, `--run-setup` was given, and the effective user is root. The message names the script and says it would run as root. The script is not run.
- A command is marked `if=`. This command does not choose a setting. The message names that block.

## The transcript

The transcript is TOML. It is written only after a shell has started, and it is written whether the run passed or a command's status did not match. One table per command that ran, in that order:

```toml
[[command]]
page = "welcome.md"
window = "bob"
block = 1
command = "echo hi\n"
status = 0
start = "2026-10-22T21:07:53.123456789Z"
end = "2026-10-22T21:07:53.456789012Z"
output = "hi\n"
```

`page` is the path listed in the index. `window` is the window the command ran in. `block` is the line of the opening fence. `command` is the fence body as written. `status` is the exit status the prompt hook reported. `start` is the UTC time just before the command is typed, and `end` is the UTC time when the hook reports the end. Both are written `YYYY-MM-DDTHH:MM:SS.NNNNNNNNNZ`. `output` is the text the command wrote, without the prompt. A command that prints its working directory shows the window folder there. A command that writes nothing has an empty output. A run that plays no command writes an empty file. No other keys are written.

A mistake in the command line exits 2. `tutorial-stepper play --help` describes the command.

The attributes are specified in [Checking a tutorial's format](format.md).
