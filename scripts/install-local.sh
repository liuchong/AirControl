#!/usr/bin/env bash
set -euo pipefail

aircontrol_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
derived_data="$aircontrol_root/.build/InstallDerivedData"
built_app="$derived_data/Build/Products/Debug/AirControl.app"
installed_app="/Applications/AirControl.app"
launch_after_install=1

if [ "${1:-}" = "--no-launch" ]; then
  launch_after_install=0
  shift
fi
if [ "$#" -ne 0 ]; then
  echo "usage: $0 [--no-launch]" >&2
  exit 2
fi

"$aircontrol_root/scripts/local-signing.sh" check >/dev/null

cd "$aircontrol_root"
xcodegen generate
xcodebuild build -quiet \
  -project AirControl.xcodeproj \
  -scheme AirControl \
  -configuration Debug \
  -destination 'platform=macOS,arch=arm64' \
  -derivedDataPath "$derived_data" \
  CODE_SIGNING_ALLOWED=NO

built_requirement="$("$aircontrol_root/scripts/local-signing.sh" sign "$built_app")"
backup_root="$(mktemp -d /private/tmp/aircontrol-install.XXXXXX)"
backup_app="$backup_root/AirControl.previous.app"
failed_app="$backup_root/AirControl.failed.app"
backup_created=0
install_complete=0

restore_previous_install() {
  local status="$?"
  if [ "$install_complete" -eq 0 ] && [ "$backup_created" -eq 1 ]; then
    if [ -e "$installed_app" ]; then
      mv "$installed_app" "$failed_app" || true
    fi
    mv "$backup_app" "$installed_app" || true
  fi
  exit "$status"
}
trap restore_previous_install EXIT

pkill -TERM -x AirControl 2>/dev/null || true
for _ in 1 2 3 4 5; do
  if ! pgrep -x AirControl >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
if pgrep -x AirControl >/dev/null 2>&1; then
  echo "AirControl did not stop; refusing to replace the running app." >&2
  exit 1
fi

if [ -e "$installed_app" ]; then
  mv "$installed_app" "$backup_app"
  backup_created=1
fi

ditto "$built_app" "$installed_app"
codesign --verify --deep --strict --verbose=2 "$installed_app"
installed_requirement="$(codesign -d -r- "$installed_app" 2>&1 | sed -n 's/^designated => /designated => /p')"
if [ "$installed_requirement" != "$built_requirement" ]; then
  echo "Installed AirControl identity differs from the verified build." >&2
  exit 1
fi

install_complete=1
trap - EXIT
echo "installed: $installed_app"
echo "designated requirement: $installed_requirement"
if [ "$backup_created" -eq 1 ]; then
  echo "recoverable previous app: $backup_app"
fi

if [ "$launch_after_install" -eq 1 ]; then
  open "$installed_app"
fi
