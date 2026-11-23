#!/bin/sh
# Install Gafira for the current user on Linux (spec 046). Builds nothing.
#
#   packaging/linux/install.sh [BINARY]   install BINARY (default: target/release/gafira of this repo)
#   packaging/linux/install.sh --uninstall
#
# Installs the binary to ~/.local/bin, the icons of assets/app-icon/hicolor
# (written by `cargo xtask gen-icons`) to the user's hicolor theme, the
# desktop entry and the .gaf MIME type, then refreshes the desktop caches
# when their tools are present.
set -eu

[ -n "${HOME:-}" ] || { echo "install.sh: HOME is not set" >&2; exit 1; }

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repo=$(CDPATH='' cd -- "$here/../.." && pwd)
data=${XDG_DATA_HOME:-$HOME/.local/share}
bin_dir=$HOME/.local/bin
icons=$data/icons/hicolor
sizes="16 24 32 48 64 128 256 512"
app=gafira
doc=application-vnd.gafira.project+zip
kit=application-vnd.gafira.brandkit+zip

# The icon files, relative to assets/app-icon/hicolor and to $icons alike.
icon_files() {
    for size in $sizes; do
        echo "${size}x${size}/apps/$app.png"
        echo "${size}x${size}/mimetypes/$doc.png"
        echo "${size}x${size}/mimetypes/$kit.png"
    done
    echo "scalable/apps/$app.svg"
    echo "scalable/mimetypes/$doc.svg"
    echo "scalable/mimetypes/$kit.svg"
}

refresh() {
    # A missing or failing cache tool never fails the install.
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database -q "$data/applications" || true
    fi
    if command -v update-mime-database >/dev/null 2>&1; then
        update-mime-database "$data/mime" >/dev/null 2>&1 || true
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -q -f -t "$icons" || true
    fi
}

uninstall() {
    rm -f "$bin_dir/$app" \
        "$data/applications/$app.desktop" \
        "$data/mime/packages/$app.xml"
    for file in $(icon_files); do
        rm -f "$icons/$file"
    done
    refresh
    echo "Gafira removed for $(id -un)."
}

install_all() {
    binary=${1:-$repo/target/release/$app}
    if [ ! -x "$binary" ]; then
        echo "install.sh: no executable at $binary" >&2
        echo "Build it first: cargo build --release --locked -p gafira" >&2
        exit 1
    fi
    source_icons=$repo/assets/app-icon/hicolor
    for file in $(icon_files); do
        [ -f "$source_icons/$file" ] || {
            echo "install.sh: missing $source_icons/$file; run cargo xtask gen-icons" >&2
            exit 1
        }
    done

    install -D -m 755 "$binary" "$bin_dir/$app"
    for file in $(icon_files); do
        install -D -m 644 "$source_icons/$file" "$icons/$file"
    done
    # The entry runs the installed binary by its full path, so the menu works
    # even when ~/.local/bin is not in the session's PATH.
    mkdir -p "$data/applications"
    sed "s|^Exec=$app |Exec=$bin_dir/$app |" "$here/gafira.desktop" >"$data/applications/$app.desktop"
    chmod 644 "$data/applications/$app.desktop"
    install -D -m 644 "$here/gafira-mime.xml" "$data/mime/packages/$app.xml"
    refresh

    echo "Gafira installed: $bin_dir/$app"
    case ":${PATH:-}:" in
        *":$bin_dir:"*) ;;
        *) echo "Note: $bin_dir is not in PATH; the menu entry works, the gafira command needs it." ;;
    esac
}

case "${1:-}" in
    --uninstall)
        [ $# -eq 1 ] || { echo "usage: install.sh [BINARY] | --uninstall" >&2; exit 2; }
        uninstall
        ;;
    -h | --help)
        echo "usage: install.sh [BINARY] | --uninstall"
        ;;
    -*)
        echo "usage: install.sh [BINARY] | --uninstall" >&2
        exit 2
        ;;
    *)
        [ $# -le 1 ] || { echo "usage: install.sh [BINARY] | --uninstall" >&2; exit 2; }
        install_all "${1:-}"
        ;;
esac
