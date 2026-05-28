#!/usr/bin/env bash
set -euo pipefail

session="codex_scroll_repro"
pane_lines=25
history_capture=4000
tmp_dir="/tmp/codex-scroll-repro"
mkdir -p "$tmp_dir"
out_file="$tmp_dir/capture.txt"
missing_file="$tmp_dir/missing.txt"
do_scroll=1
mode="codex_like"

if [[ "${1:-}" == "--no-scroll" ]]; then
  do_scroll=0
fi
if [[ "${1:-}" == "--plain" ]] || [[ "${2:-}" == "--plain" ]]; then
  mode="plain"
fi

tmux kill-session -t "$session" 2>/dev/null || true
tmux new-session -d -s "$session" -x 120 -y "$pane_lines"

# Run an emitter that mimics Codex-style scroll-region writes while continuously
# outputting numbered lines.
tmux send-keys -t "$session" "bash -lc '
  set -euo pipefail
  # initial filler
  for i in \$(seq 1 40); do printf \"FILLER-%04d\\n\" \"\$i\"; done
  mode=\"$mode\"
  for i in \$(seq 1 400); do
    if [ \"\$mode\" = \"codex_like\" ]; then
      printf \"\\033[1;20r\"      # SetScrollRegion(1..20)
      printf \"\\033[20;1H\"      # MoveTo region bottom
      printf \"\\r\\n\"           # newline in region => scroll
    fi
    printf \"S-%04d\\n\" \"\$i\"
    if [ \"\$mode\" = \"codex_like\" ]; then
      printf \"\\033[r\"          # ResetScrollRegion
    fi
    sleep 0.01
  done
  tmux wait-for -S codex_scroll_repro_done
' " C-m

# Let stream start, then optionally scroll up in copy mode while stream continues.
if [[ "$do_scroll" -eq 1 ]]; then
  sleep 1
  tmux copy-mode -t "$session"
  for _ in $(seq 1 60); do
    tmux send-keys -t "$session" -X cursor-up
  done
  sleep 2
  tmux send-keys -t "$session" -X cancel
fi

# Wait until the emitter signals completion.
tmux wait-for codex_scroll_repro_done

tmux capture-pane -t "$session" -S "-$history_capture" -p > "$out_file"
tmux kill-session -t "$session" 2>/dev/null || true

# Check for missing stream sequence lines.
awk '
  /^S-[0-9]{4}$/ { seen[$0]=1 }
  END {
    for (i=1; i<=400; i++) {
      line = sprintf("S-%04d", i)
      if (!(line in seen)) print line
    }
  }
' "$out_file" > "$missing_file"

missing_count="$(wc -l < "$missing_file" | tr -d " ")"
echo "capture_file=$out_file"
echo "missing_file=$missing_file"
echo "missing_count=$missing_count"
if [[ "$missing_count" -gt 0 ]]; then
  echo "missing_examples:"
  head -20 "$missing_file"
fi
