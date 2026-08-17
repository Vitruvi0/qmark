# qmark bash integration
# Install: add to ~/.bashrc →  eval "$(qmark init bash)"
#
# Cisco-style `?`: when the line ends with a space (e.g. `git mv `), pressing
# `?` opens an interactive picker of what can come next; choosing an entry
# appends it to the line. A `?` inside a word (globs like `ls file?.txt`) is
# inserted normally, so nothing breaks.
#
# Opt-out of the key binding (keeping the functions available):
#   export QMARK_NO_BIND=1   # before the eval line
# shellcheck shell=bash

__qmark_insert_qm() {
  READLINE_LINE="${READLINE_LINE:0:READLINE_POINT}?${READLINE_LINE:READLINE_POINT}"
  (( READLINE_POINT++ )) || true
}

__qmark_widget() {
  local line="${READLINE_LINE:0:READLINE_POINT}"

  # Insert a literal `?` unless we are at the end of a line that ends in a space.
  if [[ -z "$line" || "$line" != *' ' || "$READLINE_POINT" -ne "${#READLINE_LINE}" ]]; then
    __qmark_insert_qm
    return
  fi

  # Fail safe: if qmark is not on PATH, never break the user's prompt.
  if ! command -v qmark >/dev/null 2>&1; then
    __qmark_insert_qm
    return
  fi

  printf '\n'
  local sel
  sel="$(qmark suggest --interactive -- "$line")"
  if [[ -n "$sel" ]]; then
    READLINE_LINE="${line}${sel} "
    READLINE_POINT=${#READLINE_LINE}
  fi
}

if [[ "${QMARK_NO_BIND:-0}" != 1 && $- == *i* ]]; then
  bind -x '"?": __qmark_widget'
fi
