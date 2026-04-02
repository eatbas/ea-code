#!/usr/bin/env sh
# macOS/Linux clang wrapper for cc-rs.
# On macOS, Xcode command-line tools provide clang by default.

TOOL="clang"

if [ -n "$MAESTRO_LLVM_BIN" ] && [ -x "$MAESTRO_LLVM_BIN/$TOOL" ]; then
  exec "$MAESTRO_LLVM_BIN/$TOOL" "$@"
fi

if command -v "$TOOL" >/dev/null 2>&1; then
  exec "$TOOL" "$@"
fi

# Homebrew LLVM (Apple Silicon)
if [ -x "/opt/homebrew/opt/llvm/bin/$TOOL" ]; then
  exec "/opt/homebrew/opt/llvm/bin/$TOOL" "$@"
fi

# Homebrew LLVM (Intel Mac)
if [ -x "/usr/local/opt/llvm/bin/$TOOL" ]; then
  exec "/usr/local/opt/llvm/bin/$TOOL" "$@"
fi

echo "Maestro could not locate $TOOL. Install Xcode command-line tools (xcode-select --install) or Homebrew LLVM, or set MAESTRO_LLVM_BIN." >&2
exit 1
