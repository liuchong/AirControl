#!/usr/bin/env bash
set -euo pipefail

aircontrol_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$aircontrol_root"

cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash -n scripts/local-signing.sh
bash -n scripts/install-local.sh
bash -n integration_test/stable_local_signing.sh
xcodegen generate
/usr/libexec/PlistBuddy -c 'Print :NSCameraUsageDescription' Config/Info.plist >/dev/null
test "$(/usr/libexec/PlistBuddy -c 'Print :com.apple.security.device.camera' Config/AirControl.entitlements)" = true
test -f AirControl/Assets.xcassets/AppIcon.appiconset/icon_512x512@2x.png
test "$(sips -g pixelWidth AirControl/Assets.xcassets/AppIcon.appiconset/icon_512x512@2x.png 2>/dev/null | awk '/pixelWidth/ { print $2 }')" = 1024
test -f AirControl/UI/AirControlBrandMark.swift
grep -q 'AirControlMenuBarIcon(runState: model.runState)' AirControl/App/AirControlApp.swift
grep -q 'docs/assets/aircontrol-app-icon.png' README.md
xcodebuild build-for-testing -quiet \
  -project AirControl.xcodeproj \
  -scheme AirControl \
  -destination 'platform=macOS,arch=arm64' \
  -derivedDataPath .build/DerivedData \
  CODE_SIGNING_ALLOWED=NO
xcodebuild test -quiet \
  -project AirControl.xcodeproj \
  -scheme AirControl \
  -destination 'platform=macOS,arch=arm64' \
  -derivedDataPath .build/DerivedData \
  CODE_SIGNING_ALLOWED=NO
