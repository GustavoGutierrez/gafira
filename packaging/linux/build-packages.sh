#!/bin/sh
# Build the Linux packages of Gafira (spec 078): a .deb for Ubuntu/Debian
# and an AppImage, both in dist/.
#
#   packaging/linux/build-packages.sh            build the release binary, then both
#   packaging/linux/build-packages.sh --deb      only the .deb
#   packaging/linux/build-packages.sh --appimage only the AppImage
#
# The .deb installs the binary to /usr/bin, the desktop entry, the .gaf MIME
# type and the application and document icons of assets/app-icon/hicolor
# (written by `cargo xtask gen-icons`); dpkg's triggers refresh the desktop,
# MIME and icon caches. The AppImage is packed with appimagetool (see
# appimagetool_path).
#
# APPIMAGE_RUNTIME is the runtime of the AppImage; without it, appimagetool
# fetches the current one.
# GAFIRA_MAINTAINER overrides the Maintainer field of the .deb.
set -eu

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repo=$(CDPATH='' cd -- "$here/../.." && pwd)
app=gafira
doc=application-vnd.gafira.project+zip
kit=application-vnd.gafira.brandkit+zip
pack=application-vnd.gafira.gafpack+zip
sizes="16 24 32 48 64 128 256 512"
dist=$repo/dist
work=$repo/target/packaging
binary=$repo/target/release/$app
maintainer=${GAFIRA_MAINTAINER:-Gustavo Gutierrez <noreply@gafira.invalid>}

version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$repo/Cargo.toml" | head -n 1)
[ -n "$version" ] || { echo "build-packages.sh: no version in Cargo.toml" >&2; exit 1; }
# Debian orders 0.1.0~beta.2 before 0.1.0; a hyphen would mean a revision.
deb_version=$(echo "$version" | sed 's/-/~/')

build_binary() {
    (cd "$repo" && cargo xtask gen-icons --check && cargo build --release --locked -p gafira)
    strip --strip-debug -o "$work/$app" "$binary"
}

# The files every package carries, under $1 (a /usr prefix).
lay_out() {
    prefix=$1
    install -D -m 755 "$work/$app" "$prefix/bin/$app"
    install -D -m 644 "$here/gafira.desktop" "$prefix/share/applications/$app.desktop"
    install -D -m 644 "$here/gafira-mime.xml" "$prefix/share/mime/packages/$app.xml"
    icons=$repo/assets/app-icon/hicolor
    for size in $sizes; do
        for context in "apps/$app" "mimetypes/$doc" "mimetypes/$kit" "mimetypes/$pack"; do
            install -D -m 644 "$icons/${size}x${size}/$context.png" \
                "$prefix/share/icons/hicolor/${size}x${size}/$context.png"
        done
    done
    install -D -m 644 "$icons/scalable/apps/$app.svg" "$prefix/share/icons/hicolor/scalable/apps/$app.svg"
    for name in $doc $kit $pack; do
        install -D -m 644 "$icons/scalable/mimetypes/$name.svg" \
            "$prefix/share/icons/hicolor/scalable/mimetypes/$name.svg"
    done
    install -D -m 644 "$repo/LICENSE" "$prefix/share/doc/$app/copyright"
    install -D -m 644 "$repo/THIRD_PARTY_NOTICES.md" "$prefix/share/doc/$app/THIRD_PARTY_NOTICES.md"
}

build_deb() {
    root=$work/deb
    rm -rf "$root"
    lay_out "$root/usr"
    size=$(du -sk "$root/usr" | cut -f1)
    mkdir -p "$root/DEBIAN"
    cat >"$root/DEBIAN/control" <<EOF
Package: $app
Version: $deb_version
Architecture: amd64
Maintainer: $maintainer
Installed-Size: $size
Section: graphics
Priority: optional
Depends: libc6 (>= 2.35), libgcc-s1, libgl1 | libegl1, libxkbcommon0, libx11-6, libxcursor1, libxrandr2, libxi6
Recommends: libwayland-client0, libwayland-egl1, libxkbcommon-x11-0, xdg-desktop-portal
Description: Design editor
 Gafira is a desktop editor for social media posts, presentations,
 print and video designs. Projects are saved as .gaf files; brand kits are shared as .gafkit files and asset packages are installed from .gafpack files.
EOF
    mkdir -p "$dist"
    out=$dist/${app}_${deb_version}_amd64.deb
    dpkg-deb --root-owner-group --build "$root" "$out" >/dev/null
    echo "deb: $out"
}

# appimagetool: $APPIMAGETOOL, the one in PATH, or the official build in
# target/packaging/tools (https://github.com/AppImage/appimagetool).
appimagetool_path() {
    if [ -n "${APPIMAGETOOL:-}" ]; then
        echo "$APPIMAGETOOL"
    elif command -v appimagetool >/dev/null 2>&1; then
        command -v appimagetool
    elif [ -x "$work/tools/appimagetool-x86_64.AppImage" ]; then
        echo "$work/tools/appimagetool-x86_64.AppImage"
    fi
}

build_appimage() {
    tool=$(appimagetool_path)
    [ -n "$tool" ] || {
        echo "build-packages.sh: no appimagetool; put it in PATH or in $work/tools/appimagetool-x86_64.AppImage" >&2
        exit 1
    }
    appdir=$work/AppDir
    rm -rf "$appdir"
    lay_out "$appdir/usr"
    # The binary links only libc and libgcc (the graphics libraries are
    # the system's, loaded at run time), so nothing is bundled: the AppDir
    # is the /usr tree, its entry and icon at the root, and AppRun.
    cp "$appdir/usr/share/applications/$app.desktop" "$appdir/$app.desktop"
    cp "$appdir/usr/share/icons/hicolor/256x256/apps/$app.png" "$appdir/$app.png"
    ln -s "$app.png" "$appdir/.DirIcon"
    cat >"$appdir/AppRun" <<'RUN'
#!/bin/sh
here=$(dirname -- "$(readlink -f -- "$0")")
exec "$here/usr/bin/gafira" "$@"
RUN
    chmod 755 "$appdir/AppRun"
    mkdir -p "$dist"
    out=$dist/Gafira-$version-x86_64.AppImage
    # Extract-and-run: no FUSE needed to run the tool itself.
    ARCH=x86_64 VERSION=$version "$tool" --appimage-extract-and-run \
        ${APPIMAGE_RUNTIME:+--runtime-file "$APPIMAGE_RUNTIME"} "$appdir" "$out" >"$work/appimagetool.log" 2>&1 || {
        echo "build-packages.sh: appimagetool failed; see $work/appimagetool.log" >&2
        exit 1
    }
    echo "AppImage: $out"
}

mkdir -p "$work"
case "${1:-}" in
    "") build_binary; build_deb; build_appimage ;;
    --deb) build_binary; build_deb ;;
    --appimage) build_binary; build_appimage ;;
    *) echo "usage: build-packages.sh [--deb | --appimage]" >&2; exit 2 ;;
esac
