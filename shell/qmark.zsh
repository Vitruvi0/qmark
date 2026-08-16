# qmark zsh integration
# Install: add to ~/.zshrc →  eval "$(qmark init zsh)"
#
# Cisco-style `?`: when the line ends with a space (e.g. `git `), pressing `?`
# shows contextual help for what you have typed so far. A `?` inside a word
# (globs like `ls file?.txt`) is inserted normally, so nothing breaks.
#
# Opt-out of the key binding (keeping the functions available):
#   export QMARK_NO_BIND=1   # before the eval line

qmark-widget() {
  emulate -L zsh

  # Insert a literal `?` unless we are at the end of a line that ends in a space.
  if [[ -z "$LBUFFER" || "$LBUFFER" != *' ' || -n "$RBUFFER" ]]; then
    zle self-insert
    return
  fi

  # Fail safe: if qmark is not on PATH, never break the user's prompt.
  if ! command -v qmark >/dev/null 2>&1; then
    zle self-insert
    return
  fi

  zle -I
  print -r --
  qmark suggest -- "$LBUFFER"
  zle redisplay
}

if [[ "${QMARK_NO_BIND:-0}" != 1 ]]; then
  zle -N qmark-widget
  bindkey '?' qmark-widget
fi
