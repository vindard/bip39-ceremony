#!/usr/bin/env bash
set -euo pipefail

command -v guix >/dev/null || {
    echo "guix is required; enter the pinned Nix development shell" >&2
    exit 1
}

output="$({
    printf '%s\n' '(use-modules (guix gexp))'
    printf '%s\n' '(for-each (lambda (file) (call-with-input-file file (lambda (port) (let loop ((form (read port))) (unless (eof-object? form) (loop (read port))))))) (list "contrib/guix/channels.scm" "contrib/guix/package.scm"))'
    printf '%s\n' '(display "Guix Scheme syntax parsed\n")'
    printf '%s\n' ',quit'
} | guix repl 2>&1)"

grep -qx 'Guix Scheme syntax parsed' <<<"$output" || {
    printf '%s\n' "$output" >&2
    exit 1
}

echo "Guix Scheme syntax parsed"
