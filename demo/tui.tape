# devwatch TUI demo
# Requires: cargo build -p tui (first time)
# Run with: vhs demo/tui.tape

Output demo/tui.gif

Set Shell "zsh"
Set FontSize 13
Set Width 1200
Set Height 600
Set Theme "Catppuccin Mocha"
Set TypingSpeed 60ms
Set PlaybackSpeed 1.0

Hide
Type "cargo build -p tui 2>&1 | tail -1 && echo BUILD_DONE"
Enter
Sleep 30s
Show

Type "./target/debug/devwatch-tui --demo"
Enter

# Hold on initial state — show PR list + pre-populated Event Log entries
Sleep 4s

# Wait for the demo polling pulse (~8 s cycle) to flash "Polling…" in the status bar
Sleep 6s

# ── Navigate ──────────────────────────────────────────────────────────────────
Down
Sleep 300ms
Down
Sleep 300ms
Down
Sleep 300ms
Down
Sleep 300ms
Down
Sleep 500ms
Up
Sleep 300ms
Up
Sleep 800ms

# ── Sort (s cycles columns, S reverses direction) ─────────────────────────────
Type "s"
Sleep 700ms
Type "s"
Sleep 700ms
Type "s"
Sleep 700ms
Type "S"
Sleep 800ms
# Clear sort by cycling past all columns
Type "s"
Sleep 300ms
Type "s"
Sleep 300ms
Type "s"
Sleep 300ms
Type "s"
Sleep 800ms

# ── Filter ────────────────────────────────────────────────────────────────────
Type "/"
Sleep 400ms
Type "open"
Sleep 1200ms
Escape        # exits filter input mode → back to Normal (text still set)
Sleep 400ms
Escape        # clears filter text in Normal mode
Sleep 600ms

# ── Column reorder ────────────────────────────────────────────────────────────
Type "o"
Sleep 600ms
Right
Sleep 500ms
Right
Sleep 500ms
Type "H"
Sleep 500ms
Type "H"
Sleep 700ms
Escape
Sleep 1s

# ── Add demo PRs quickly so all three toasts stack ────────────────────────────
Type "n"
Sleep 600ms
Type "n"
Sleep 800ms
Type "n"
# Hold so the viewer can read all three stacked toasts, then watch them expire
Sleep 5s

# ── Cycle notification mode (m) — indicator updates in status bar ──────────────
Type "m"
Sleep 1200ms
Type "m"
Sleep 1200ms
Type "m"
Sleep 1200ms
Type "m"
Sleep 800ms

# ── Quit ──────────────────────────────────────────────────────────────────────
Type "q"
