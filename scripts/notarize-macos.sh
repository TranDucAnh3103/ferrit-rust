#!/usr/bin/env bash
set -euo pipefail

DMG_PATH="${1:-target/universal-apple-darwin/release/bundle/dmg/GitUpload.dmg}"
APPLE_ID="${APPLE_ID:?APPLE_ID required}"
TEAM_ID="${APPLE_TEAM_ID:?APPLE_TEAM_ID required}"
APP_PASSWORD="${APPLE_APP_SPECIFIC_PASSWORD:?APPLE_APP_SPECIFIC_PASSWORD required}"

xcrun notarytool submit "$DMG_PATH" \
  --apple-id "$APPLE_ID" \
  --team-id "$TEAM_ID" \
  --password "$APP_PASSWORD" \
  --wait

xcrun stapler staple "$DMG_PATH"
xcrun stapler validate "$DMG_PATH"
