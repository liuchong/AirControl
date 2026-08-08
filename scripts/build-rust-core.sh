#!/usr/bin/env bash
set -euo pipefail

configuration="${1:-Debug}"
architectures="${2:-$(uname -m)}"
profile="debug"
release_flag=""
if [[ "$configuration" == "Release" ]]; then
  profile="release"
  release_flag="--release"
fi

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
output_dir="$repo_root/.build/rust/$configuration"
mkdir -p "$output_dir"

libraries=()
for architecture in $architectures; do
  case "$architecture" in
    arm64) rust_target="aarch64-apple-darwin" ;;
    x86_64) rust_target="x86_64-apple-darwin" ;;
    *) echo "Unsupported macOS architecture: $architecture" >&2; exit 2 ;;
  esac
  cargo build --manifest-path "$repo_root/Cargo.toml" --package aircontrol-core --target "$rust_target" $release_flag
  libraries+=("$repo_root/target/$rust_target/$profile/libaircontrol_core.a")
done

destination="$output_dir/libaircontrol_core.a"
if [[ ${#libraries[@]} -eq 1 ]]; then
  cp "${libraries[0]}" "$destination"
else
  xcrun lipo -create "${libraries[@]}" -output "$destination"
fi
