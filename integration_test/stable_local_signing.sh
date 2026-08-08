#!/usr/bin/env bash
set -euo pipefail

aircontrol_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
signing_tool="$aircontrol_root/scripts/local-signing.sh"
expected_identifier="com.liuchong.AirControl"
expected_identity="${AIRCONTROL_SIGN_IDENTITY:-AirControl Local Code Signing}"

if [ ! -x "$signing_tool" ]; then
  echo "missing executable signing tool: $signing_tool" >&2
  exit 1
fi

test_root="$(mktemp -d "${TMPDIR:-/tmp}/aircontrol-signing-test.XXXXXX")"
trap 'rm -rf "$test_root"' EXIT

make_test_app() {
  local app_path="$1"
  local identifier="$2"
  local version_text="$3"
  mkdir -p "$app_path/Contents/MacOS"
  swiftc -o "$app_path/Contents/MacOS/AirControl" - <<SWIFT
import Foundation
print("$version_text")
SWIFT
  /usr/libexec/PlistBuddy -c 'Add :CFBundleExecutable string AirControl' "$app_path/Contents/Info.plist"
  /usr/libexec/PlistBuddy -c "Add :CFBundleIdentifier string $identifier" "$app_path/Contents/Info.plist"
  /usr/libexec/PlistBuddy -c 'Add :CFBundlePackageType string APPL' "$app_path/Contents/Info.plist"
}

if AIRCONTROL_SIGN_IDENTITY="AirControl Missing Identity For Test" "$signing_tool" check >/dev/null 2>&1; then
  echo "missing signing identity unexpectedly passed" >&2
  exit 1
fi

invalid_app="$test_root/Invalid.app"
make_test_app "$invalid_app" "com.example.invalid" "invalid"
if AIRCONTROL_SIGN_IDENTITY="$expected_identity" "$signing_tool" sign "$invalid_app" >/dev/null 2>&1; then
  echo "invalid bundle identifier unexpectedly passed" >&2
  exit 1
fi

first_app="$test_root/First.app"
second_app="$test_root/Second.app"
make_test_app "$first_app" "$expected_identifier" "first-build"
make_test_app "$second_app" "$expected_identifier" "second-build-with-different-code"

AIRCONTROL_SIGN_IDENTITY="$expected_identity" "$signing_tool" sign "$first_app"
AIRCONTROL_SIGN_IDENTITY="$expected_identity" "$signing_tool" sign "$second_app"

first_requirement="$(codesign -d -r- "$first_app" 2>&1 | sed -n 's/^designated => /designated => /p')"
second_requirement="$(codesign -d -r- "$second_app" 2>&1 | sed -n 's/^designated => /designated => /p')"

if [ -z "$first_requirement" ] || [ "$first_requirement" != "$second_requirement" ]; then
  echo "designated requirements are missing or unstable" >&2
  echo "first:  $first_requirement" >&2
  echo "second: $second_requirement" >&2
  exit 1
fi

if [[ "$first_requirement" == *"cdhash"* ]]; then
  echo "designated requirement is still bound to a build hash: $first_requirement" >&2
  exit 1
fi

codesign --verify --deep --strict --verbose=2 "$first_app"
codesign --verify --deep --strict --verbose=2 "$second_app"
echo "$first_requirement"
