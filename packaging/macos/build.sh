#!/usr/bin/env bash
# Build the macOS package of Gafira (spec 090) into dist/: a .dmg with the
# application, signed ad hoc (it is not notarized). On macOS only.
set -euo pipefail

cd "$(dirname "$0")/../.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
[ -n "$version" ] || { echo "build.sh: no version in Cargo.toml" >&2; exit 1; }
cargo build --release --locked -p gafira
stage=target/packaging/macos
app=$stage/Gafira.app
rm -rf "$stage"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" dist
cp target/release/gafira "$app/Contents/MacOS/gafira"
cp assets/app-icon/macos/*.icns LICENSE THIRD_PARTY_NOTICES.md "$app/Contents/Resources/"
# A bundle version has numbers alone.
sed "s/@VERSION@/${version%%-*}/g" packaging/macos/Info.plist >"$app/Contents/Info.plist"
codesign --force --deep --sign - "$app"
ln -s /Applications "$stage/Applications"
out=dist/Gafira-$version-macos-$(uname -m).dmg
hdiutil create -volname Gafira -srcfolder "$stage" -ov -format UDZO "$out" >/dev/null
echo "macos: $out"
