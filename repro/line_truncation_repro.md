# Line truncation repro (issue #15380)

This document captures a deterministic reproduction of missing `S-####` lines in terminal output history.

## Context

- Reported symptom: a line can disappear from the middle of visible output while scrolling through prior assistant output.
- Reporter note: same symptom is also seen in **Windows Terminal without tmux**.
- Repro here uses tmux because it is deterministic in this environment.

## Repro script

Use:

```bash
./repro/tmux_scrollback_repro.sh
```

The script:

1. Starts a fresh tmux session.
2. Emits filler lines.
3. Emits numbered stream lines (`S-0001` to `S-0400`) using a Codex-like history insertion escape pattern.
4. Captures pane history and checks for missing sequence entries.

## Expected vs observed

- Expected: no missing `S-####` lines.
- Observed (deterministic in this environment): one missing middle line (commonly `S-0391`).

Example output:

```text
capture_file=/tmp/codex-scroll-repro/capture.txt
missing_file=/tmp/codex-scroll-repro/missing.txt
missing_count=1
missing_examples:
S-0391
```

Control run (plain line output path) does not show the gap:

```bash
./repro/tmux_scrollback_repro.sh --plain
```

Expected/observed for control:

- `missing_count=0`
