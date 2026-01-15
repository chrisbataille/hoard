# Hoard Future Features

## ✅ Real-time Usage Tracking (IMPLEMENTED)

Shell hooks for real-time command tracking have been implemented. See `docs/design/REALTIME_USAGE_TRACKING.md` for details.

**New commands:**
- `hoards usage log <cmd>` - Log a single command (called by shell hooks)
- `hoards usage init [shell]` - Show hook setup instructions
- `hoards usage config [--mode]` - View/change tracking mode (scan/hook)
- `hoards usage reset [-f]` - Reset usage counters

---

## Previous Future Features

## Real-time Usage Tracking (Shell Hook)

When ready to implement, add a `preexec` hook for real-time command logging:

### Fish
```fish
# In config.fish
function __hoard_log_cmd --on-event fish_preexec
    set -l cmd (string split ' ' $argv[1])[1]
    hoard log-usage $cmd &>/dev/null &
end
```

### Bash
```bash
# In .bashrc
preexec() {
    local cmd="${1%% *}"
    hoard log-usage "$cmd" &>/dev/null &
}
```

### Zsh
```zsh
# In .zshrc
preexec() {
    local cmd="${1%% *}"
    hoard log-usage "$cmd" &>/dev/null &
}
```

This would provide:
- Real-time tracking (not dependent on history file updates)
- More accurate timestamps
- Immediate feedback in `hoard usage` command
