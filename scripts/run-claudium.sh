#!/bin/sh
# Launch Claudium with your Microsoft sign-in configured.
# Put your client ID (one line, no quotes) in client-id.txt next to this file.
here="$(cd "$(dirname "$0")" && pwd)"
if [ ! -f "$here/client-id.txt" ]; then
  echo "Create $here/client-id.txt with your CLAUDIUM_MSA_CLIENT_ID on one line." >&2
  exit 1
fi
export CLAUDIUM_MSA_CLIENT_ID="$(tr -d ' \r\n' < "$here/client-id.txt")"
exec "$here/Claudium-Launcher" "$@"
