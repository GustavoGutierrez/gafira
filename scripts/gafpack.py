#!/usr/bin/env python3
"""Build a Gafira asset package (`.gafpack`, spec 097) from a folder.

The script reads the resources of a folder, writes `manifest.json` (format
1.1), the package thumbnail `thumb.webp` and a small preview of each image,
and packs everything as Gafira installs it. It applies the same rules as
Gafira before writing anything, so a package it builds installs.

Usage:

    python3 scripts/gafpack.py build SOURCE OUTPUT.gafpack --type clipart \\
        --id com.studio.christmas --name-es "Clip art navideño" \\
        --name-en "Christmas Clip Art" --description-es "..." \\
        --description-en "..." --author "Studio" --license CC0-1.0

    python3 scripts/gafpack.py check PACKAGE.gafpack

Signatures (spec 112): a package signed with a publisher key Gafira trusts
shows as "verified author" in Gafira. The private key never goes in a
repository; keep a backup of it.

    python3 scripts/gafpack.py keygen KEYS_DIR/studio-2026.ed25519
    python3 scripts/gafpack.py sign PACKAGE.gafpack --key KEY --publisher "Studio"
    python3 scripts/gafpack.py build ... --sign-key KEY     # build and sign

The key id is the key file's name without `.ed25519` unless --key-id says
otherwise; --key can also come from the GAFPACK_SIGN_KEY variable. Signing
and checking signatures need the `cryptography` package.

The package type decides what each file becomes:

    clipart, illustrations, icons, shapes   PNG, WebP and SVG files.
    frames      PNG, WebP and SVG (decorative); with --frame-slots, each SVG
                is a frame that takes a photo, cut by its outline.
    powerclips  SVG masks; each becomes a PowerClip (cover or contain).
    collages    JSON descriptors {"slots": [{"x","y","w","h"}, ...]} in
                shares of the page, or a ready "gafira.collage" descriptor.
    templates   .gaf template files saved from Gafira.
    mixed       Subfolders named after the types above (illustrations/,
                clipart/, icons/, shapes/, frames/, powerclips/, collages/,
                templates/); each file takes its folder's type.

Optional files in SOURCE:

    metadata.json  Per file overrides, keyed by the file's path relative to
                   SOURCE: {"santa.webp": {"name": "Papá Noel", "tags":
                   ["navidad"], "keywords": ["santa"]}}.
    LICENSE.txt    The licence text; required for a LicenseRef-* licence.
    thumb.webp|png|jpg  The package thumbnail; without it, a mosaic of the
                   first images is made.

Requires Pillow with WebP support (pip install pillow).
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import re
import sys
import unicodedata
import xml.etree.ElementTree as ElementTree
import zipfile
from dataclasses import dataclass, field
from pathlib import Path

try:
    from PIL import Image, features
except ImportError:  # pragma: no cover - checked at start
    Image = None
    features = None

SIGNATURE_PATH = "META-INF/signature.json"
SIGNATURE_FORMAT = "gafpack-signature"
SIGNATURE_VERSION = 1
# The publisher keys Gafira trusts: key id -> (publisher, Ed25519 public key).
# Keep in step with TRUSTED_KEYS in crates/gafira-ui/src/pack_signature.rs.
TRUSTED_KEYS = {
    "gafira-2026": ("Gafira", "92d2192dad58b85572600dd5c540e07fef79787d063f73bc84f2e57aad5aff48"),
}

FORMAT = "gafpack"
FORMAT_VERSION = "1.1"
THUMBNAIL = "thumb.webp"
THUMB_SIDE = 1024
THUMB_SIDES = range(256, 2049)
THUMB_MOST_BYTES = 2 * 1024 * 1024
ASSET_MOST_SIDE = 8192
PREVIEW_SIDE = 256
MOST_ENTRIES = 20_000
MOST_BYTES = 4 * 1024 * 1024 * 1024
MOST_FILE = 256 * 1024 * 1024
MOST_SLOTS = 24

PACKAGE_TYPES = {
    # package type: (asset type, section)
    "clipart": ("clipart", "illustrations"),
    "illustrations": ("illustration", "illustrations"),
    "icons": ("icon", "icons"),
    "shapes": ("shape", "shapes"),
    "frames": ("frame", "frames"),
    "powerclips": ("powerclip", "powerclips"),
    "collages": ("collage", "collages"),
    "templates": ("template", "templates"),
}
FOLDER_TYPES = {name: value for name, value in PACKAGE_TYPES.items()}
FOLDER_TYPES.update({"illustration": PACKAGE_TYPES["illustrations"], "icon": PACKAGE_TYPES["icons"]})
IMAGE_EXTENSIONS = {".png", ".webp", ".svg"}
SKIP = {"metadata.json", "license.txt", "thumb.webp", "thumb.png", "thumb.jpg", "thumb.jpeg"}


class PackError(Exception):
    """A rule the package breaks."""


@dataclass
class Asset:
    id: str
    type: str
    section: str
    name: str
    source: Path
    file: str | None = None
    definition: str | None = None
    preview: str | None = None
    tags: list[str] = field(default_factory=list)
    keywords: list[str] = field(default_factory=list)
    extra: dict[str, bytes] = field(default_factory=dict)
    width: int | None = None
    height: int | None = None
    hash: str | None = None


# --- Rules shared with Gafira (crates/gafira-ui/src/packs.rs) -------------


def safe_path(path: str) -> bool:
    parts = path.split("/")
    return (
        bool(path)
        and len(path) <= 512
        and not path.startswith("/")
        and "\\" not in path
        and ":" not in path
        and not any(ord(c) < 32 or ord(c) == 127 for c in path)
        and len(parts) <= 8
        and all(p and p not in (".", "..") for p in parts)
    )


def valid_id(value: str) -> bool:
    parts = value.split(".")
    return len(value) <= 128 and len(parts) >= 2 and all(re.fullmatch(r"[a-z0-9_-]+", p) for p in parts)


def valid_version(value: str) -> bool:
    return re.fullmatch(r"\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?", value) is not None


def svg_problem(data: bytes) -> str | None:
    """Why an SVG is refused, or None: it must parse and hold nothing active
    or external."""
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        return "the SVG is not UTF-8"
    lower = text.lower()
    if "<!doctype" in lower or "<!entity" in lower:
        return "the SVG has a DOCTYPE or entities"
    try:
        root = ElementTree.fromstring(text)
    except ElementTree.ParseError as error:
        return f"the SVG does not parse ({error})"
    if root.tag.rsplit("}", 1)[-1] != "svg":
        return "the root is not <svg>"
    active = {"script", "foreignobject", "iframe", "embed", "object", "audio", "video"}
    for node in root.iter():
        tag = node.tag.rsplit("}", 1)[-1].lower()
        if tag in active:
            return f"the SVG has <{tag}>"
        for name, value in node.attrib.items():
            local = name.rsplit("}", 1)[-1].lower()
            value_l = value.strip().lower()
            if local.startswith("on"):
                return f"the SVG has an event attribute ({local})"
            if local == "href" and not (value_l.startswith("#") or value_l.startswith("data:image/")):
                return "the SVG links outside itself"
            if "url(" in value_l and not _inside(value_l):
                return "the SVG loads an outside url()"
        if tag == "style" and node.text and ("@import" in node.text or not _inside(node.text.lower())):
            return "the SVG style loads outside resources"
    return None


def _inside(text: str) -> bool:
    return all(part.lstrip("'\" ").startswith("#") for part in text.split("url(")[1:])


def image_info(data: bytes, kind: str) -> tuple[int, int]:
    """The size of a PNG or WebP, after checking it decodes within limits."""
    image = Image.open(io.BytesIO(data))
    expected = {".png": "PNG", ".webp": "WEBP"}[kind]
    if image.format != expected:
        raise PackError(f"is {image.format}, not {expected}")
    width, height = image.size
    if not (0 < width <= ASSET_MOST_SIDE and 0 < height <= ASSET_MOST_SIDE):
        raise PackError(f"is {width}x{height}; the most is {ASSET_MOST_SIDE} a side")
    image.load()
    return width, height


# --- Building ---------------------------------------------------------------


def slug(text: str) -> str:
    text = unicodedata.normalize("NFKD", text).encode("ascii", "ignore").decode()
    return re.sub(r"[^a-z0-9_-]+", "-", text.lower()).strip("-") or "asset"


def nice_name(stem: str) -> str:
    words = re.sub(r"^[0-9]+[_ -]+", "", stem)
    words = re.sub(r"[_-]+", " ", words).strip()
    return (words[:1].upper() + words[1:]) if words else stem


def archive_name(text: str) -> str:
    """A file name that is safe inside the package."""
    text = unicodedata.normalize("NFC", text)
    text = re.sub(r"[\\/:\x00-\x1f\x7f]", "-", text).strip(" .")
    return text or "file"


def collect(source: Path, package_type: str) -> list[tuple[Path, tuple[str, str]]]:
    """Every resource file of `source` with its (asset type, section)."""
    found = []
    if package_type == "mixed":
        for folder in sorted(p for p in source.iterdir() if p.is_dir()):
            kind = FOLDER_TYPES.get(folder.name.lower())
            if kind is None:
                print(f"  skipped folder {folder.name}/ (not a type)", file=sys.stderr)
                continue
            for path in sorted(folder.rglob("*")):
                if path.is_file() and not path.name.startswith("."):
                    found.append((path, kind))
    else:
        kind = PACKAGE_TYPES[package_type]
        for path in sorted(source.rglob("*")):
            if path.is_file() and not path.name.startswith(".") and path.relative_to(source).as_posix().lower() not in SKIP:
                found.append((path, kind))
    return found


def make_assets(args, source: Path, metadata: dict) -> list[Asset]:
    assets: list[Asset] = []
    used_ids: set[str] = set()
    used_paths: set[str] = set()
    base_tags = [t.strip() for t in (args.tags or "").split(",") if t.strip()]
    for path, (kind, section) in collect(source, args.type):
        relative = path.relative_to(source).as_posix()
        ext = path.suffix.lower()
        meta = metadata.get(relative, {})
        stem = path.stem
        ident = slug(meta.get("id", relative.rsplit(".", 1)[0]))
        while ident in used_ids:
            ident += "-2"
        used_ids.add(ident)
        name = meta.get("name") or nice_name(stem)
        folder_tags = [p for p in path.relative_to(source).parent.parts if p and p.lower() not in FOLDER_TYPES]
        asset = Asset(
            id=ident,
            type=kind,
            section=section,
            name=name[:200],
            source=path,
            tags=list(dict.fromkeys(base_tags + folder_tags + meta.get("tags", []))),
            keywords=meta.get("keywords", []),
        )
        packed = f"assets/{section}/{archive_name(relative.replace('/', '-'))}"
        while packed.lower() in used_paths:
            packed = packed.rsplit(".", 1)[0] + "-2." + packed.rsplit(".", 1)[1]
        used_paths.add(packed.lower())
        if kind in ("clipart", "illustration", "icon", "shape") or (kind == "frame" and not (args.frame_slots and ext == ".svg")):
            if ext not in IMAGE_EXTENSIONS:
                print(f"  skipped {relative} (not PNG, WebP or SVG)", file=sys.stderr)
                used_ids.discard(ident)
                continue
            asset.file = packed
        elif kind in ("frame", "powerclip"):
            if ext != ".svg":
                print(f"  skipped {relative} (a {kind} is an SVG mask)", file=sys.stderr)
                used_ids.discard(ident)
                continue
            schema = "gafira.frame" if kind == "frame" else "gafira.powerclip"
            mask = packed.rsplit("/", 1)[1]
            descriptor = {"schema": schema, "version": 1, "mask": mask}
            if kind == "powerclip":
                descriptor["contentFit"] = meta.get("contentFit", args.content_fit)
            asset.definition = packed.rsplit(".", 1)[0] + ".json"
            asset.extra[asset.definition] = json.dumps(descriptor).encode()
            asset.extra[packed] = path.read_bytes()
        elif kind == "collage":
            if ext != ".json":
                print(f"  skipped {relative} (a collage is a JSON descriptor)", file=sys.stderr)
                used_ids.discard(ident)
                continue
            data = json.loads(path.read_text(encoding="utf-8"))
            descriptor = {
                "schema": "gafira.collage",
                "version": 1,
                "gap": data.get("gap", 0.012),
                "slots": data["slots"],
            }
            asset.definition = packed
            asset.extra[packed] = json.dumps(descriptor).encode()
        elif kind == "template":
            if ext != ".gaf":
                print(f"  skipped {relative} (a template is a .gaf file)", file=sys.stderr)
                used_ids.discard(ident)
                continue
            asset.file = packed
        assets.append(asset)
    if not assets:
        raise PackError("no resources found for this package type")
    return assets


def _prepare(job: tuple[str, str, str, bool]) -> tuple:
    """Reads and checks one file, and makes its preview: run in a worker
    process. Returns (file, width, height, sha256, preview path, preview
    bytes) or raises PackError; the file itself is copied from disk later,
    so it is never held in memory past this check."""
    source, packed, name, previews = job
    data = Path(source).read_bytes()
    if len(data) > MOST_FILE:
        raise PackError(f"{name}: larger than {MOST_FILE // 2**20} MiB")
    ext = Path(packed).suffix.lower()
    width = height = None
    preview_path = preview = None
    if ext == ".svg":
        problem = svg_problem(data)
        if problem:
            raise PackError(f"{name}: {problem}")
    elif ext in (".png", ".webp"):
        try:
            width, height = image_info(data, ext)
        except PackError as error:
            raise PackError(f"{name}: {error}") from None
        except Exception as error:  # noqa: BLE001 - any decode failure
            raise PackError(f"{name}: does not decode ({error})") from None
        if previews:
            image = Image.open(io.BytesIO(data))
            image.draft("RGBA", (PREVIEW_SIDE, PREVIEW_SIDE))
            image.thumbnail((PREVIEW_SIDE, PREVIEW_SIDE), Image.BILINEAR)
            out = io.BytesIO()
            image.save(out, "WEBP", quality=80, method=3)
            preview_path = "previews/" + packed.split("/", 2)[2].rsplit(".", 1)[0] + ".webp"
            preview = out.getvalue()
    elif ext == ".gaf" and not zipfile.is_zipfile(io.BytesIO(data)):
        raise PackError(f"{name}: is not a .gaf file")
    return packed, width, height, hashlib.sha256(data).hexdigest(), preview_path, preview


def check_assets(
    assets: list[Asset], previews: bool, workers: int
) -> tuple[dict[str, bytes], dict[str, Path]]:
    """Reads, checks and prepares every file, a few at a time in worker
    processes; the small files made here and the source files to copy."""
    from concurrent.futures import ProcessPoolExecutor

    files: dict[str, bytes] = {}
    copies: dict[str, Path] = {}
    jobs = [(str(a.source), a.file, a.source.name, previews) for a in assets if a.file]
    by_file = {a.file: a for a in assets if a.file}
    done = 0
    with ProcessPoolExecutor(max_workers=workers) as pool:
        for packed, width, height, digest, preview_path, preview in pool.map(_prepare, jobs, chunksize=4):
            asset = by_file[packed]
            asset.width, asset.height, asset.hash = width, height, digest
            copies[packed] = asset.source
            if preview_path:
                asset.preview = preview_path
                files[preview_path] = preview
            done += 1
            if sys.stderr.isatty():
                print(f"\r  checked {done}/{len(jobs)}", end="", file=sys.stderr, flush=True)
    if jobs and sys.stderr.isatty():
        print(file=sys.stderr)
    for asset in assets:
        where = asset.source.name
        for path, data in asset.extra.items():
            if path.endswith(".svg"):
                problem = svg_problem(data)
                if problem:
                    raise PackError(f"{where}: {problem}")
            if path.endswith(".json") and asset.type == "collage":
                slots = json.loads(data)["slots"]
                if not 1 <= len(slots) <= MOST_SLOTS:
                    raise PackError(f"{where}: a collage has 1 to {MOST_SLOTS} slots")
                for slot in slots:
                    x, y, w, h = (float(slot[k]) for k in ("x", "y", "w", "h"))
                    if not (0 <= x <= 1 and 0 <= y <= 1 and w > 0 and h > 0 and x + w <= 1 + 1e-9 and y + h <= 1 + 1e-9):
                        raise PackError(f"{where}: a slot is outside the page")
            files[path] = data
    return files, copies


def make_thumbnail(source: Path, assets: list[Asset], given: str | None) -> bytes:
    candidates = [Path(given)] if given else [source / n for n in ("thumb.webp", "thumb.png", "thumb.jpg", "thumb.jpeg")]
    for candidate in candidates:
        if candidate.is_file():
            image = Image.open(candidate).convert("RGBA")
            side = min(max(min(image.size), 256), 2048)
            # Centred square crop: never stretched.
            w, h = image.size
            square = min(w, h)
            image = image.crop(((w - square) // 2, (h - square) // 2, (w - square) // 2 + square, (h - square) // 2 + square))
            image = image.resize((side, side), Image.LANCZOS)
            return _webp(image)
    # A mosaic of up to 16 images on Gafira's warm white.
    pictures = [a.source for a in assets if a.file and Path(a.file).suffix.lower() in (".png", ".webp")]
    thumb = Image.new("RGBA", (THUMB_SIDE, THUMB_SIDE), (250, 250, 247, 255))
    if pictures:
        step = max(1, len(pictures) // 16)
        picks = pictures[::step][:16]
        per_row = 4 if len(picks) > 9 else 3 if len(picks) > 4 else 2 if len(picks) > 1 else 1
        cell = THUMB_SIDE // per_row
        for k, path in enumerate(picks):
            image = Image.open(path).convert("RGBA")
            image.thumbnail((cell - 24, cell - 24))
            x = (k % per_row) * cell + (cell - image.width) // 2
            y = (k // per_row) * cell + (cell - image.height) // 2
            thumb.alpha_composite(image, (x, y))
    return _webp(thumb)


def _webp(image) -> bytes:
    out = io.BytesIO()
    image.convert("RGB").save(out, "WEBP", quality=86, method=6)
    data = out.getvalue()
    if len(data) > THUMB_MOST_BYTES:
        out = io.BytesIO()
        image.convert("RGB").save(out, "WEBP", quality=70, method=6)
        data = out.getvalue()
    return data


def build(args) -> None:
    source = Path(args.source)
    if not source.is_dir():
        raise PackError(f"{source} is not a folder")
    for value, field_name, low, high in (
        (args.name_es, "--name-es", 1, 100),
        (args.name_en, "--name-en", 1, 100),
        (args.description_es, "--description-es", 10, 1000),
        (args.description_en, "--description-en", 10, 1000),
    ):
        if not low <= len(value.strip()) <= high:
            raise PackError(f"{field_name} needs {low} to {high} characters")
    if not valid_id(args.id):
        raise PackError("--id must be lowercase parts joined by dots, like com.studio.pack")
    for value, flag in ((args.version, "--version"), (args.min_gafira, "--min-gafira")):
        if not valid_version(value):
            raise PackError(f"{flag} must be a version like 1.0.0")
    if not args.author.strip():
        raise PackError("--author is required")
    metadata_path = source / "metadata.json"
    metadata = json.loads(metadata_path.read_text(encoding="utf-8")) if metadata_path.is_file() else {}
    print(f"Reading {source} ...")
    assets = make_assets(args, source, metadata)
    workers = args.workers or max(1, (os.cpu_count() or 2) - 1)
    files, copies = check_assets(assets, not args.no_previews, workers)
    sections = sorted({a.section for a in assets})
    if args.type != "mixed" and PACKAGE_TYPES[args.type][1] not in sections:
        raise PackError(f"a {args.type} package needs {PACKAGE_TYPES[args.type][1]} assets")
    license_file = None
    license_text = Path(args.license_file) if args.license_file else source / "LICENSE.txt"
    if license_text.is_file():
        license_file = "licenses/LICENSE.txt"
        files[license_file] = license_text.read_bytes()
    elif args.license.startswith("LicenseRef-"):
        raise PackError("a LicenseRef-* licence needs its text (--license-file or LICENSE.txt)")
    thumbnail = make_thumbnail(source, assets, args.thumbnail)
    files[THUMBNAIL] = thumbnail
    manifest = {
        "format": FORMAT,
        "formatVersion": FORMAT_VERSION,
        "id": args.id,
        "version": args.version,
        "packageType": args.type,
        "categories": sections,
        "name": {"es": args.name_es.strip(), "en": args.name_en.strip()},
        "description": {"es": args.description_es.strip(), "en": args.description_en.strip()},
        "author": {k: v for k, v in (("name", args.author), ("url", args.author_url), ("email", args.author_email)) if v},
        "thumbnail": THUMBNAIL,
        "license": args.license,
        "minGafiraVersion": args.min_gafira,
        "tags": [t.strip() for t in (args.tags or "").split(",") if t.strip()],
        "assets": [],
    }
    if license_file:
        manifest["licenseFile"] = license_file
    if args.homepage:
        manifest["homepage"] = args.homepage
    for asset in assets:
        entry = {"id": asset.id, "type": asset.type, "name": asset.name, "placement": asset.section}
        for key in ("file", "definition", "preview"):
            if getattr(asset, key):
                entry[key] = getattr(asset, key)
        if asset.tags:
            entry["tags"] = asset.tags
        if asset.keywords:
            entry["keywords"] = asset.keywords
        if asset.width:
            entry["width"], entry["height"] = asset.width, asset.height
        if asset.hash:
            entry["hash"] = asset.hash
        manifest["assets"].append(entry)
    files["manifest.json"] = json.dumps(manifest, ensure_ascii=False, indent=2).encode()
    total = sum(len(b) for b in files.values()) + sum(p.stat().st_size for p in copies.values())
    if len(files) + len(copies) > MOST_ENTRIES or total > MOST_BYTES:
        raise PackError("the package is past Gafira's limits")
    for path in [*files, *copies]:
        if not safe_path(path):
            raise PackError(f"unsafe path {path}")
    output = Path(args.output)
    partial = output.with_name(output.name + ".part")
    stored = (".webp", ".png", ".gaf")
    with zipfile.ZipFile(partial, "w") as archive:
        def kind(path: str) -> int:
            return zipfile.ZIP_STORED if path.lower().endswith(stored) else zipfile.ZIP_DEFLATED

        for path in ["manifest.json", THUMBNAIL] + sorted(p for p in files if p not in ("manifest.json", THUMBNAIL)):
            archive.writestr(path, files[path], compress_type=kind(path))
        # The originals, streamed from disk.
        for path in sorted(copies):
            archive.write(copies[path], path, compress_type=kind(path))
    os.replace(partial, output)
    counts: dict[str, int] = {}
    for asset in assets:
        counts[asset.section] = counts.get(asset.section, 0) + 1
    print(f"Wrote {output} ({output.stat().st_size / 2**20:.1f} MB)")
    for section, n in sorted(counts.items()):
        print(f"  {n} {section}")
    if args.sign_key:
        sign(str(output), args.sign_key, args.author, args.key_id)
    states: list[str] = []
    problems = check(output, states)
    if problems:
        raise PackError("the written package fails its own check: " + "; ".join(problems))
    print("Valid: it installs in Gafira.")
    print(f"Signature: {states[0]}")


def _ed25519():
    try:
        from cryptography.hazmat.primitives.asymmetric import ed25519
    except ImportError as error:
        raise PackError("signatures need the cryptography package: pip install cryptography") from error
    return ed25519


def signed_message(key_id: str, publisher: str, files: dict[str, str]) -> bytes:
    """The text a signature signs: the same bytes Gafira rebuilds."""
    text = f"{SIGNATURE_FORMAT}/{SIGNATURE_VERSION}\nkey:{key_id}\npublisher:{publisher}\n"
    for path in sorted(files, key=lambda p: p.encode()):
        text += f"{files[path]}  {path}\n"
    return text.encode()


def signed_path(path: str) -> bool:
    return not path.startswith("META-INF/")


def load_key(path: str):
    seed = bytes.fromhex(Path(path).read_text(encoding="utf-8").strip())
    if len(seed) != 32:
        raise PackError(f"{path} is not an Ed25519 key (64 hex digits)")
    return _ed25519().Ed25519PrivateKey.from_private_bytes(seed)


def key_id_of(path: str) -> str:
    name = Path(path).name
    return name[: -len(".ed25519")] if name.endswith(".ed25519") else Path(path).stem


def keygen(path: str) -> None:
    from cryptography.hazmat.primitives import serialization as ser

    target = Path(path)
    if target.exists():
        raise PackError(f"{target} already exists; never overwrite a signing key")
    target.parent.mkdir(parents=True, exist_ok=True)
    key = _ed25519().Ed25519PrivateKey.generate()
    seed = key.private_bytes(ser.Encoding.Raw, ser.PrivateFormat.Raw, ser.NoEncryption()).hex()
    public = key.public_key().public_bytes(ser.Encoding.Raw, ser.PublicFormat.Raw).hex()
    fd = os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "w") as out:
        out.write(seed + "\n")
    Path(str(target) + ".pub").write_text(public + "\n", encoding="utf-8")
    print(f"Wrote {target} (private, keep a backup) and {target}.pub")
    print(f"key id: {key_id_of(path)}\npublic: {public}")
    print("Gafira verifies it once the key is in TRUSTED_KEYS (Rust and this script).")


def sign(package: str, key_path: str, publisher: str | None, key_id: str | None = None) -> None:
    """Signs `package` in place: every file but META-INF/ is hashed and the
    signature is written to META-INF/signature.json (an old one is replaced)."""
    key = load_key(key_path)
    key_id = key_id or key_id_of(key_path)
    path = Path(package)
    partial = path.with_name(path.name + ".part")
    files: dict[str, str] = {}
    with zipfile.ZipFile(path) as source:
        manifest = json.loads(source.read("manifest.json"))
        author = manifest.get("author", {}).get("name", "").strip()
        publisher = (publisher or author).strip()
        if author != publisher:
            raise PackError(f"the manifest author is {author!r}, not the publisher {publisher!r}")
        trusted = TRUSTED_KEYS.get(key_id)
        if trusted and trusted[0] != publisher:
            raise PackError(f"key {key_id} belongs to {trusted[0]!r}, not {publisher!r}")
        with zipfile.ZipFile(partial, "w") as out:
            for info in source.infolist():
                if info.is_dir() or not signed_path(info.filename):
                    continue
                data = source.read(info)
                files[info.filename] = hashlib.sha256(data).hexdigest()
                out.writestr(info.filename, data, compress_type=info.compress_type)
            signature = key.sign(signed_message(key_id, publisher, files)).hex()
            note = {
                "format": SIGNATURE_FORMAT,
                "version": SIGNATURE_VERSION,
                "algorithm": "ed25519",
                "keyId": key_id,
                "publisher": publisher,
                "files": files,
                "signature": signature,
            }
            out.writestr(SIGNATURE_PATH, json.dumps(note, ensure_ascii=False, indent=1), compress_type=zipfile.ZIP_DEFLATED)
    os.replace(partial, path)
    print(f"Signed {path} as {publisher} (key {key_id}, {len(files)} files)")


def signature_state(archive: zipfile.ZipFile, names: set[str], author: str) -> tuple[str, list[str]]:
    """Who signed the package, as Gafira decides it (spec 112): the state
    ("unsigned", "unknown key ...", "verified ...") and its problems."""
    if SIGNATURE_PATH not in names:
        return "unsigned: Gafira shows the author as declared, not verified", []
    try:
        note = json.loads(archive.read(SIGNATURE_PATH))
    except ValueError:
        return "invalid", ["the signature does not read"]
    if (note.get("format"), note.get("version"), note.get("algorithm")) != (SIGNATURE_FORMAT, SIGNATURE_VERSION, "ed25519"):
        return "invalid", ["unknown signature format, version or algorithm"]
    files = {n: hashlib.sha256(archive.read(n)).hexdigest() for n in names if signed_path(n)}
    if note.get("files") != files:
        return "invalid", ["the package changed after it was signed"]
    key_id = note.get("keyId", "")
    if key_id not in TRUSTED_KEYS:
        return f"signed with key {key_id!r}, which Gafira does not know: not verified", []
    publisher, public = TRUSTED_KEYS[key_id]
    ed25519 = _ed25519()
    try:
        ed25519.Ed25519PublicKey.from_public_bytes(bytes.fromhex(public)).verify(
            bytes.fromhex(note.get("signature", "")),
            signed_message(key_id, note.get("publisher", ""), note.get("files", {})),
        )
    except Exception:  # noqa: BLE001 - any failure is a bad signature
        return "invalid", ["the signature does not verify"]
    if note.get("publisher") != publisher:
        return "invalid", ["the signed publisher is not the key's"]
    if author.strip() != publisher:
        return "invalid", [f"author {author!r} is not the signer {publisher!r}"]
    return f"verified: signed by {publisher} (key {key_id})", []


def check(path: Path, report: list[str] | None = None) -> list[str]:
    """The problems of an existing package, as Gafira would see them (the
    template contents excepted: Gafira reads those itself)."""
    problems: list[str] = []
    try:
        archive = zipfile.ZipFile(path)
    except zipfile.BadZipFile:
        return ["not a ZIP file"]
    with archive:
        names = set()
        for info in archive.infolist():
            if info.is_dir():
                continue
            if info.flag_bits & 0x1:
                problems.append(f"{info.filename}: encrypted")
            if not safe_path(info.filename):
                problems.append(f"{info.filename}: unsafe path")
            if info.filename.lower() in {n.lower() for n in names}:
                problems.append(f"{info.filename}: repeated")
            names.add(info.filename)
        if "manifest.json" not in names:
            return problems + ["manifest.json is missing"]
        manifest = json.loads(archive.read("manifest.json"))
        for key, value in (("format", FORMAT), ("formatVersion", FORMAT_VERSION), ("thumbnail", THUMBNAIL)):
            if manifest.get(key) != value:
                problems.append(f"{key} must be {value!r}")
        if not valid_id(manifest.get("id", "")):
            problems.append("invalid id")
        for lang in ("es", "en"):
            if not 1 <= len(manifest.get("name", {}).get(lang, "").strip()) <= 100:
                problems.append(f"name.{lang} is missing or too long")
            if not 10 <= len(manifest.get("description", {}).get(lang, "").strip()) <= 1000:
                problems.append(f"description.{lang} needs 10 to 1000 characters")
        if not manifest.get("author", {}).get("name", "").strip():
            problems.append("author.name is missing")
        if THUMBNAIL in names:
            thumb = Image.open(io.BytesIO(archive.read(THUMBNAIL)))
            if thumb.format != "WEBP" or getattr(thumb, "is_animated", False):
                problems.append("thumb.webp must be a still WebP")
            elif thumb.size[0] != thumb.size[1] or thumb.size[0] not in THUMB_SIDES:
                problems.append("thumb.webp must be square, 256 to 2048 px")
        else:
            problems.append("thumb.webp is missing")
        ids = set()
        sections = set()
        for asset in manifest.get("assets", []):
            if asset.get("id") in ids:
                problems.append(f"{asset.get('id')}: repeated id")
            ids.add(asset.get("id"))
            sections.add(asset.get("placement"))
            for key in ("file", "definition", "preview"):
                part = asset.get(key)
                if part and part not in names:
                    problems.append(f"{asset.get('id')}: {part} is missing")
            if asset.get("hash") and asset.get("file") in names:
                if hashlib.sha256(archive.read(asset["file"])).hexdigest() != asset["hash"]:
                    problems.append(f"{asset.get('id')}: hash does not match")
        if set(manifest.get("categories", [])) != sections:
            problems.append("categories do not match the assets")
        state, signed = signature_state(archive, names, manifest.get("author", {}).get("name", ""))
        problems.extend(signed)
        if report is not None:
            report.append(state)
    return problems


def report(path: Path) -> int:
    states: list[str] = []
    problems = check(path, states)
    for p in problems:
        print(f"  - {p}")
    print("Valid." if not problems else f"{len(problems)} problems.")
    if states:
        print(f"Signature: {states[0]}")
    return 0 if not problems else 1


def main() -> int:
    parser = argparse.ArgumentParser(description="Build or check a Gafira asset package (.gafpack).")
    sub = parser.add_subparsers(dest="command", required=True)
    b = sub.add_parser("build", help="pack a folder of resources")
    b.add_argument("source", help="the folder of the resources")
    b.add_argument("output", help="the .gafpack to write")
    b.add_argument("--type", required=True, choices=[*PACKAGE_TYPES, "mixed"], help="the package type")
    b.add_argument("--id", required=True, help="persistent id, like com.studio.christmas")
    b.add_argument("--version", default="1.0.0", help="package version (SemVer)")
    b.add_argument("--name-es", required=True)
    b.add_argument("--name-en", required=True)
    b.add_argument("--description-es", required=True, help="10 to 1000 characters")
    b.add_argument("--description-en", required=True, help="10 to 1000 characters")
    b.add_argument("--author", required=True, help="author name, as declared")
    b.add_argument("--author-url")
    b.add_argument("--author-email")
    b.add_argument("--license", required=True, help="SPDX id (CC0-1.0, CC-BY-4.0...) or LicenseRef-*")
    b.add_argument("--license-file", help="licence text; default SOURCE/LICENSE.txt")
    b.add_argument("--min-gafira", default="0.1.0-beta.4", help="the oldest Gafira it works in")
    b.add_argument("--tags", help="comma-separated tags for every asset")
    b.add_argument("--homepage")
    b.add_argument("--thumbnail", help="an image for thumb.webp; default a mosaic")
    b.add_argument("--no-previews", action="store_true", help="do not make small previews")
    b.add_argument("--frame-slots", action="store_true", help="frames: each SVG takes a photo")
    b.add_argument("--content-fit", default="cover", choices=["cover", "contain"], help="powerclips")
    b.add_argument("--workers", type=int, help="processes that check files; default the cores less one")
    b.add_argument("--sign-key", help="sign the package with this Ed25519 key file")
    b.add_argument("--key-id", help="the key id; default the key file name without .ed25519")
    c = sub.add_parser("check", help="check an existing .gafpack, its signature included")
    c.add_argument("package")
    k = sub.add_parser("keygen", help="make an Ed25519 signing key (never overwrites)")
    k.add_argument("key", help="where to write the private key, like KEYS_DIR/studio-2026.ed25519")
    s = sub.add_parser("sign", help="sign an existing .gafpack in place")
    s.add_argument("package")
    s.add_argument("--key", default=os.environ.get("GAFPACK_SIGN_KEY"), help="the Ed25519 key file (or GAFPACK_SIGN_KEY)")
    s.add_argument("--key-id", help="the key id; default the key file name without .ed25519")
    s.add_argument("--publisher", help="who signs; must be the manifest author (default: it)")
    args = parser.parse_args()
    if args.command == "keygen":
        try:
            keygen(args.key)
        except PackError as error:
            print(f"error: {error}", file=sys.stderr)
            return 1
        return 0
    if Image is None or not features.check("webp"):
        print("Pillow with WebP support is needed: pip install pillow", file=sys.stderr)
        return 2
    try:
        if args.command == "build":
            build(args)
        elif args.command == "sign":
            if not args.key:
                raise PackError("--key (or GAFPACK_SIGN_KEY) is required")
            sign(args.package, args.key, args.publisher, args.key_id)
            return report(Path(args.package))
        else:
            return report(Path(args.package))
    except PackError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
