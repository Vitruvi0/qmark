# qmark zsh integration
# Install: add to ~/.zshrc →  eval "$(qmark init zsh)"
#
# Cisco-style `?`: when the line ends with a space (e.g. `git mv `), pressing
# `?` keeps the `?` visible on the line and opens an interactive picker of what
# can come next; choosing an entry replaces the `?` with it. Esc removes the
# `?` and leaves the line as it was. A `?` inside a word (globs like
# `ls file?.txt`) is inserted normally, so nothing breaks.
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

  local line="$LBUFFER" sel
  # Cisco-style feedback: show the `?` on the line while the picker is up.
  LBUFFER+='?'
  zle -R
  zle -I
  print -r --
  sel="$(qmark suggest --interactive -- "$line")"
  if [[ -n "$sel" ]]; then
    LBUFFER="${line}${sel} "
  else
    LBUFFER="$line"
  fi
  zle redisplay
}

if [[ "${QMARK_NO_BIND:-0}" != 1 ]]; then
  zle -N qmark-widget
  bindkey '?' qmark-widget
fi
