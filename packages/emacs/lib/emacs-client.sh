#!/bin/bash
# Shared bash helpers for emacsclient operations.
# Source this from other scripts:
#   source "$(dirname "$0")/../lib/emacs-client.sh"

# Escape a string for safe inclusion inside an elisp double-quoted literal.
# Escapes backslashes first, then double quotes.
emacs_escape() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    printf '%s' "$s"
}

# Check Emacs daemon is running. Returns 0 if up, 1 otherwise.
emacs_daemon_running() {
    emacsclient --eval "t" >/dev/null 2>&1
}

# Require that the daemon is running, exit 1 with message otherwise.
emacs_require_daemon() {
    if ! emacs_daemon_running; then
        echo "Error: Emacs daemon not running. Start with: emacs --daemon" >&2
        exit 1
    fi
}

# Load an elisp package from a directory if not already loaded.
# Args: PKG_NAME LOAD_PATH
# Side effect: emacs daemon now has PKG required.
emacs_ensure_pkg() {
    local pkg="$1"
    local path="$2"

    if emacsclient --eval "(featurep '${pkg})" 2>/dev/null | grep -q "^t$"; then
        return 0
    fi

    emacsclient --eval "(progn
        (add-to-list 'load-path \"$(emacs_escape "$path")\")
        (require '${pkg}))" >/dev/null || {
        echo "Error: Failed to load elisp package '${pkg}' from '${path}'" >&2
        return 1
    }
}
