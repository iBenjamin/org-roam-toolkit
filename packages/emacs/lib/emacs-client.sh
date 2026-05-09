#!/bin/bash
# Shared bash helpers for emacsclient operations.
# Source this from other scripts:
#   source "$(dirname "$0")/../lib/emacs-client.sh"

# Default timeouts (seconds). Probes are short so callers with their own
# wrapper-level deadline (e.g. dashboard probe at 5s) never have to SIGKILL
# us — that would orphan emacsclient grandchildren to PID 1.
: "${EMACS_PROBE_TIMEOUT:=2}"
: "${EMACS_EVAL_TIMEOUT:=30}"

# PID of the most recent backgrounded emacsclient. The trap installed by
# bin/emacs-eval reads this on EXIT/INT/TERM to make sure we don't leave
# emacsclient processes parked on a hung daemon's socket.
_EMACS_CHILD_PID=""

# Escape a string for safe inclusion inside an elisp double-quoted literal.
# Escapes backslashes first, then double quotes.
emacs_escape() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    printf '%s' "$s"
}

# Run emacsclient with a hard wall-clock timeout. Portable across bash 3.2
# (no `wait -n`) and macOS without coreutils' timeout(1).
# Args: TIMEOUT_SECS [emacsclient args...]
# Returns: emacsclient's exit code, or 124 if the timeout fired.
#
# Implementation note: every potential non-zero exit is `|| true`-fenced so
# `set -e` callers don't abort here — `wait` on a signaled child returns
# 128+signum, and `kill` on an already-exited watcher returns 1. Both are
# expected control flow, not errors.
_emacs_run() {
    local secs="$1"; shift
    emacsclient "$@" &
    _EMACS_CHILD_PID=$!
    # IMPORTANT: redirect the watcher subshell's I/O to /dev/null. When we
    # `kill $watcher`, only the subshell's bash receives SIGTERM — its
    # forked `sleep` orphans and keeps holding any file descriptors it
    # inherited. Without this redirect, the orphaned sleep would hold our
    # stdout pipe open for the full timeout window, and any reader
    # (e.g. dashboard probe) would block on EOF until the deadline.
    (
        sleep "$secs"
        kill "$_EMACS_CHILD_PID" 2>/dev/null || true
    ) </dev/null >/dev/null 2>&1 &
    local watcher=$!
    local rc=0
    wait "$_EMACS_CHILD_PID" 2>/dev/null || rc=$?
    kill "$watcher" 2>/dev/null || true
    wait "$watcher" 2>/dev/null || true
    _EMACS_CHILD_PID=""
    # SIGTERM-killed (128+15=143) means we hit the watchdog deadline.
    [[ $rc -eq 143 ]] && return 124
    return "$rc"
}

# Kill any in-flight emacsclient child. Safe to call from a trap.
_emacs_kill_child() {
    if [[ -n "$_EMACS_CHILD_PID" ]] && kill -0 "$_EMACS_CHILD_PID" 2>/dev/null; then
        kill "$_EMACS_CHILD_PID" 2>/dev/null || true
    fi
}

# Check Emacs daemon is running. Returns 0 if up, 1 otherwise.
emacs_daemon_running() {
    _emacs_run "$EMACS_PROBE_TIMEOUT" --eval "t" >/dev/null 2>&1
}

# Require that the daemon is running, exit 1 with message otherwise.
emacs_require_daemon() {
    if ! emacs_daemon_running; then
        echo "Error: Emacs daemon not running or unresponsive (timed out after ${EMACS_PROBE_TIMEOUT}s). Start with: emacs --daemon" >&2
        exit 1
    fi
}

# Load an elisp package from a directory if not already loaded.
# Args: PKG_NAME LOAD_PATH
# Side effect: emacs daemon now has PKG required.
emacs_ensure_pkg() {
    local pkg="$1"
    local path="$2"

    if _emacs_run "$EMACS_EVAL_TIMEOUT" --eval "(featurep '${pkg})" 2>/dev/null | grep -q "^t$"; then
        return 0
    fi

    _emacs_run "$EMACS_EVAL_TIMEOUT" --eval "(progn
        (add-to-list 'load-path \"$(emacs_escape "$path")\")
        (require '${pkg}))" >/dev/null || {
        echo "Error: Failed to load elisp package '${pkg}' from '${path}'" >&2
        return 1
    }
}
