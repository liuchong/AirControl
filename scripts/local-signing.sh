#!/usr/bin/env bash
set -euo pipefail

aircontrol_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
signing_identity="${AIRCONTROL_SIGN_IDENTITY:-AirControl Local Code Signing}"
signing_identifier="${AIRCONTROL_SIGN_IDENTIFIER:-com.liuchong.AirControl}"

usage() {
  echo "usage: $0 check | sign <AirControl.app>" >&2
}

identity_hash() {
  security find-identity -v -p codesigning \
    | awk -v identity="$signing_identity" -F '"' '$2 == identity { gsub(/^[[:space:]]*[0-9]+\)[[:space:]]*/, "", $1); print $1 }'
}

check_identity() {
  local hashes
  hashes="$(identity_hash)"
  if [ -z "$hashes" ]; then
    echo "AirControl signing identity is unavailable: $signing_identity" >&2
    echo "Create or restore the fixed local identity; refusing to use an ad-hoc signature." >&2
    return 1
  fi
  if [ "$(printf '%s\n' "$hashes" | sed '/^$/d' | wc -l | tr -d ' ')" != "1" ]; then
    echo "AirControl signing identity is ambiguous: $signing_identity" >&2
    return 1
  fi
  printf '%s\n' "$hashes"
}

sign_app() {
  local app_path="$1"
  local info_plist="$app_path/Contents/Info.plist"
  local bundle_identifier
  local requirement

  check_identity >/dev/null
  if [ ! -d "$app_path" ] || [ ! -f "$info_plist" ]; then
    echo "AirControl app bundle is invalid: $app_path" >&2
    return 1
  fi

  bundle_identifier="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$info_plist")"
  if [ "$bundle_identifier" != "$signing_identifier" ]; then
    echo "AirControl bundle identifier mismatch: got $bundle_identifier, expected $signing_identifier" >&2
    return 1
  fi

  codesign --force \
    --sign "$signing_identity" \
    --identifier "$signing_identifier" \
    --entitlements "$aircontrol_root/Config/AirControl.entitlements" \
    "$app_path"
  codesign --verify --deep --strict --verbose=2 "$app_path"

  requirement="$(codesign -d -r- "$app_path" 2>&1 | sed -n 's/^designated => /designated => /p')"
  if [ -z "$requirement" ] || [[ "$requirement" == *"cdhash"* ]]; then
    echo "AirControl received an unstable designated requirement: $requirement" >&2
    return 1
  fi
  printf '%s\n' "$requirement"
}

case "${1:-}" in
  check)
    if [ "$#" -ne 1 ]; then
      usage
      exit 2
    fi
    check_identity
    ;;
  sign)
    if [ "$#" -ne 2 ]; then
      usage
      exit 2
    fi
    sign_app "$2"
    ;;
  *)
    usage
    exit 2
    ;;
esac
