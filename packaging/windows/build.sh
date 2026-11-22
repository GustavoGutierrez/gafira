#!/usr/bin/env bash
# Build the Windows packages of Gafira (spec 090) into dist/: the installer
# and a portable .zip.
#
#   packaging/windows/build.sh
#
# On Windows (Git Bash, as in the release workflow) the target is the one of
# the machine, MSVC. On Linux it is x86_64-pc-windows-gnu: mingw-w64 and nsis
# are needed (apt install mingw-w64 nsis).
set -euo pipefail

cd "$(dirname "$0")/../.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
[ -n "$version" ] || { echo "build.sh: no version in Cargo.toml" >&2; exit 1; }
# 0.1.0-beta.3 is 0.1.0.3; a release is 0.1.0.65535, after all its betas.
base=${version%%-*}
case "$version" in
  *-*) numbers="$base.$(grep -oE '[0-9]+$' <<<"$version" || echo 0)" ;;
  *) numbers="$base.65535" ;;
esac

if [ "${OS:-}" = "Windows_NT" ]; then
  target=x86_64-pc-windows-msvc
  root=$(cygpath -w "$PWD")
  sep=(-DSEP=\\)
  # It is there on the runners of GitHub, out of the path.
  export PATH="$PATH:/c/Program Files (x86)/NSIS"
else
  target=x86_64-pc-windows-gnu
  root=$PWD
  sep=()
fi
# The C runtime goes inside: a clean Windows does not have it.
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="-C target-feature=+crt-static"
cargo build --release --locked -p gafira --target "$target"
staged=dist/windows-x64
rm -rf "$staged"
mkdir -p "$staged/legal"
cp "target/$target/release/gafira.exe" "$staged/Gafira.exe"
cp LICENSE "$staged/legal/LICENSE.txt"
cp THIRD_PARTY_NOTICES.md "$staged/legal/"
# As arguments, so that no shell takes them for paths.
MSYS2_ARG_CONV_EXCL='*' makensis -V2 -NOCD "-DAPP_VERSION=$version" \
  "-DAPP_NUMBERS=$numbers" "-DREPO_ROOT=$root" "${sep[@]}" packaging/windows/gafira.nsi
zip=Gafira-$version-windows-x64.zip
rm -f "dist/$zip"
if command -v 7z >/dev/null 2>&1; then
  (cd "$staged" && 7z a -bso0 "../$zip" .)
else
  (cd "$staged" && zip -qr "../$zip" .)
fi
echo "windows: dist/Gafira-$version-Setup-x64.exe dist/$zip"
