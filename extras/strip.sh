#!/usr/bin/env bash
set -euo pipefail

# Arguments
readonly exepath="$1"

# Retrieve the build id of the binary specified in the first argument.
function get-build-id {
    rust-objcopy --dump-section .note.gnu.build-id=/dev/stdout "$1" |
        cat |
        tail -c+17 |
        od -An -v -tx1 |
        tr -d ' \n'
}

symname="$(get-build-id "${exepath}").debug"
sympath="$(dirname "$exepath")/$symname"

# Copy all debug information from `$exepath` to `$sympath`.
echo "▶ Save debug information to $symname"
rust-objcopy --only-keep-debug --compress-debug-sections=zlib "$exepath" "$sympath"

# Remove all debug information from the original executable, and link our newly
# created `sym` file to it. This allows us to easily debug using `gdb`/`lldb`.
echo "▶ Strip debug information from $exepath"
cd "$(dirname "$sympath")" || exit 1
rust-objcopy \
    --strip-debug \
    --strip-unneeded \
    --remove-section=".gnu_debuglink" \
    --add-gnu-debuglink="$symname" \
    "$(basename "$exepath")"
cd - >/dev/null || exit 1
