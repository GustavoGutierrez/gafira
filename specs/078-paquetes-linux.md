# 078 — Paquetes de Linux (.deb y AppImage) y abrir .gaf con doble clic

> **Estado:** in-progress — 2026-11-21. Paquetes generados y probados; pendiente de instalarlos el titular.

## Objetivo

Instalar Gafira en Ubuntu y otras distribuciones con un paquete, con el icono de los proyectos `.gaf` y su asociación, y que un doble clic en un `.gaf` lo abra.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S78-01 | Icono de los `.gaf`. | El icono de documento sale de `docs/manuals/marca/assets/gafira-gaf-icon.svg` con `cargo xtask gen-icons`: SVG y PNG de 16 a 512 px en `assets/app-icon/hicolor/*/mimetypes`, idénticos al PNG de la marca a 512 px; `gen-icons --check` los da al día. |
| S78-02 | Asociación. | `gafira.desktop` declara `MimeType=application/vnd.gafira.project+zip;` y `Exec=gafira %f`; `gafira proyecto.gaf` abre el proyecto al terminar el splash. |
| S78-03 | `.deb`. | `packaging/linux/build-packages.sh --deb` crea `dist/gafira_<versión>_amd64.deb` (versión de Cargo con `~`): `/usr/bin/gafira`, la entrada, el tipo MIME, los iconos de la aplicación y del documento, la licencia y los avisos de terceros; dependencias de glibc ≥ 2.35, GL/EGL, X11 y xkbcommon, y recomienda Wayland y el portal de escritorio. Los disparadores de dpkg refrescan las cachés. |
| S78-04 | AppImage. | `build-packages.sh --appimage` crea `dist/Gafira-<versión>-x86_64.AppImage` con appimagetool (en PATH, en `APPIMAGETOOL` o en `target/packaging/tools`). El binario solo enlaza libc y libgcc, así que no se empaquetan bibliotecas. Arranca sin instalar nada. |
| S78-05 | Splash centrado donde se abre. | El splash se coloca en el centro, en horizontal y en vertical, del monitor donde está el puntero (preguntado al servidor X con x11rb y RandR: `gafira_platform::pointer_monitor`), no en el primero que lista la biblioteca de ventanas; la ventana principal crece desde ese centro (`a_window_is_centred_on_its_monitor`). Sin X (Wayland puro) lo centra el sistema. |
