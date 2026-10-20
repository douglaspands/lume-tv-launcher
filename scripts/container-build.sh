#!/usr/bin/env bash
# Builds lume inside a container, so no Rust, NDK or Android SDK is needed on the host.
#
# Usage: scripts/container-build.sh [debug|release]
#
# Runtime selection: $LUME_CONTAINER (a command), else wslc.exe (WSL), else docker.
# Results land in dist/ (APK, and AAB for release).
set -euo pipefail

variant="${1:-debug}"
case "$variant" in
  debug | release) ;;
  *) echo "usage: $0 [debug|release]" >&2; exit 2 ;;
esac

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"
mkdir -p dist
tarball="dist/artifacts-$variant.tar"

# Limits so the fat-LTO build fits in the ~7 GB a default WSL VM has.
jobs="${CARGO_BUILD_JOBS:-4}"
heap="${GRADLE_HEAP:-1536m}"

find_runtime() {
  if [[ -n "${LUME_CONTAINER:-}" ]]; then
    echo "$LUME_CONTAINER"
  elif command -v wslc.exe >/dev/null 2>&1; then
    echo "wslc.exe"
  elif [[ -x "/mnt/c/Program Files/WSL/wslc.exe" ]]; then
    echo "/mnt/c/Program Files/WSL/wslc.exe"
  elif command -v docker >/dev/null 2>&1; then
    echo "docker buildx"
  else
    echo "error: no container runtime found (install Docker, or use WSL with wslc)" >&2
    exit 1
  fi
}

runtime="$(find_runtime)"
echo "Using container runtime: $runtime"

# wslc.exe is a Windows program: it needs Windows paths.
path_for_runtime() {
  if [[ "$runtime" == *wslc.exe ]]; then wslpath -w "$1"; else echo "$1"; fi
}

# Release signing: pass the keystore and its passwords as build secrets, so they are never
# written into an image layer. Set LUME_KEYSTORE (a file) and LUME_KEYSTORE_PASSWORD,
# LUME_KEY_ALIAS and LUME_KEY_PASSWORD in the environment.
secrets=()
signed=false
if [[ "$variant" == "release" && -n "${LUME_KEYSTORE:-}" ]]; then
  : "${LUME_KEYSTORE_PASSWORD:?set LUME_KEYSTORE_PASSWORD}"
  : "${LUME_KEY_ALIAS:?set LUME_KEY_ALIAS}"
  : "${LUME_KEY_PASSWORD:?set LUME_KEY_PASSWORD}"
  [[ -f "$LUME_KEYSTORE" ]] || { echo "error: $LUME_KEYSTORE does not exist" >&2; exit 1; }
  # Passwords go through short-lived files rather than environment variables: the Windows side
  # of wslc cannot see the environment of this shell.
  secret_dir="$(mktemp -d)"
  chmod 700 "$secret_dir"
  trap 'rm -rf "$secret_dir"' EXIT
  printf '%s' "$LUME_KEYSTORE_PASSWORD" > "$secret_dir/store_password"
  printf '%s' "$LUME_KEY_ALIAS" > "$secret_dir/key_alias"
  printf '%s' "$LUME_KEY_PASSWORD" > "$secret_dir/key_password"
  secrets=(
    --secret "id=lume_keystore,src=$(path_for_runtime "$(realpath "$LUME_KEYSTORE")")"
    --secret "id=lume_store_password,src=$(path_for_runtime "$secret_dir/store_password")"
    --secret "id=lume_key_alias,src=$(path_for_runtime "$secret_dir/key_alias")"
    --secret "id=lume_key_password,src=$(path_for_runtime "$secret_dir/key_password")"
  )
  signed=true
elif [[ "$variant" == "release" ]]; then
  echo "note: LUME_KEYSTORE is not set, so the release is signed with the debug key (do not publish it)"
fi

# wslc cannot export a directory, only a tar file; the same works for docker buildx.
# shellcheck disable=SC2086 # $runtime may be two words ("docker buildx")
$runtime build \
  --progress plain \
  --build-arg "VARIANT=$variant" \
  --build-arg "CARGO_BUILD_JOBS=$jobs" \
  --build-arg "GRADLE_HEAP=$heap" \
  --build-arg "LUME_VERSION_NAME=${LUME_VERSION_NAME:-}" \
  "${secrets[@]}" \
  --target artifacts \
  --output "type=tar,dest=$(path_for_runtime "$repo/$tarball")" \
  --file "$(path_for_runtime "$repo/docker/Dockerfile")" \
  "$(path_for_runtime "$repo")"

# The artifacts stage only contains the build outputs.
tar -xmf "$tarball" -C dist
rm -f "$tarball"

version="${LUME_VERSION_NAME:-$(sed -n 's/^def lumeVersion = .*: "\(.*\)"/\1/p' android/app/build.gradle | head -n1)}"
for f in dist/app-*.apk dist/app-*.aab; do
  [[ -e "$f" ]] || continue
  extension="${f##*.}"
  suffix=""
  if [[ "$variant" == "release" && "$signed" == false ]]; then suffix="-debugsigned"; fi
  target="dist/lume-$version-$variant$suffix.$extension"
  mv "$f" "$target"
  echo "Built $target"
done
ls -l dist
