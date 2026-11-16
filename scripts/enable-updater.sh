#!/bin/sh
# One-time: turn on signed auto-updates.
# Run by the repo owner; it uploads the updater private key as a GitHub secret.
#   sh scripts/enable-updater.sh
set -eu
KEYDIR="${KOTHALIPI_KEYS:-$HOME/.kothalipi-keys}"
REPO="Kothalipi/kothalipi-desktop"
[ -f "$KEYDIR/updater.key" ] || { echo "No key at $KEYDIR/updater.key. Create one:"; echo "  bunx tauri signer generate -w $KEYDIR/updater.key"; exit 1; }
gh secret set TAURI_SIGNING_PRIVATE_KEY -R "$REPO" < "$KEYDIR/updater.key"
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD -R "$REPO" < "$KEYDIR/updater.password"
# Public key must match the private one.
PUB=$(tr -d '\n' < "$KEYDIR/updater.key.pub")
grep -q "$PUB" src-tauri/tauri.conf.json || echo "warning: src-tauri/tauri.conf.json pubkey differs from $KEYDIR/updater.key.pub"
sed -i.bak 's/"createUpdaterArtifacts": false/"createUpdaterArtifacts": true/' src-tauri/tauri.conf.json && rm -f src-tauri/tauri.conf.json.bak
echo "Done. Commit src-tauri/tauri.conf.json, then run the Release workflow."
