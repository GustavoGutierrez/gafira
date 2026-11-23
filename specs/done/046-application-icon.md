# 046 — Icono de la aplicación e instalación en el escritorio Linux

> **Estado:** done — 2026-11-17 — S46-01..S46-07 cubiertos por `xtask` (`icons::tests`, 6) y `gafira-ui` (`app_icon::tests`, 1); `cargo xtask gen-icons --check` limpio; entrada `.desktop` validada con `desktop-file-validate`; `install.sh` sin ejecutar (ver Evidencia); 3156 tests.

## Objetivo

Que Gafira tenga su icono en la ventana, en el dock o la barra de tareas y en el menú de aplicaciones de Linux, y que los archivos `.gaf` muestren su icono de documento. Los iconos salen de un generador reproducible a partir de los SVG de la marca (03 §5.3, DIS-13; manual de marca §7.12 y §12.4), nunca de exportaciones a mano.

## Alcance / no alcance

Alcance:

- Crate `xtask` (miembro del espacio de trabajo, fuera de `crates/` y `apps/`) con `cargo xtask gen-icons` (alias en `.cargo/config.toml`):
  - compone `gafira.svg`: baldosa `#FAFAF7` (`brand.canvas`) de 512 × 512 con esquinas redondeadas (radio 3/16 del lado, 96) y los trazados de `assets/gafira-logo-symbol.svg` copiados tal cual (`d`, relleno y regla de relleno), centrados al 62 % del lado según su `viewBox`;
  - rasteriza esa baldosa con resvg a 16, 24, 32, 48, 64, 128, 256 y 512 px;
  - copia el icono de documento del manual (`docs/manuals/marca/assets/gafira-gaf-icon.svg`, el original, §7.12) y lo rasteriza a los mismos tamaños;
  - escribe el árbol `hicolor` versionado en `assets/app-icon/hicolor/` (`<n>x<n>/apps/gafira.png`, `scalable/apps/gafira.svg`, `<n>x<n>/mimetypes/application-vnd.gafira.project+zip.png` y su SVG);
  - `cargo xtask gen-icons --check` regenera en memoria y falla si algún archivo versionado difiere;
  - el generador rechaza un símbolo que no sabe copiar fielmente (`viewBox` que no empieza en 0 0, elementos distintos de `path`, trazados con `transform`).
- Ventana (`gafira-ui`, `shell::run`): `ViewportBuilder::with_icon` con el PNG de 256 px incrustado (`app_icon.rs`) y `with_app_id("gafira")`; si el PNG no se decodifica, la ventana abre con el icono del sistema y el error va al registro.
- Linux, en `packaging/linux/`:
  - `gafira.desktop`: `Name=Gafira`, `Icon=gafira`, `Categories=Graphics;`, `StartupWMClass=gafira`, comentario con el eslogan en ES y EN;
  - `gafira-mime.xml`: el tipo `application/vnd.gafira.project+zip` (00-decisiones, 02 §7.1) con el glob `*.gaf`, subclase de `application/zip` y su icono;
  - `install.sh`: no compila; instala para el usuario el binario dado (por defecto `target/release/gafira` del repositorio) en `~/.local/bin/gafira`, los iconos en `${XDG_DATA_HOME:-~/.local/share}/icons/hicolor/`, la entrada en `applications/` con `Exec=` a la ruta completa del binario instalado y el tipo MIME en `mime/packages/gafira.xml`; después ejecuta `update-desktop-database`, `update-mime-database` y `gtk-update-icon-cache` si existen, sin fallar si fallan; `--uninstall` lo quita todo.

No alcance:

- `gafira.ico` de Windows y `gafira.icns` de macOS (03 §5.3), ni el icono del instalador MSI o del DMG.
- La versión simplificada del símbolo para 16–32 px (03 §5.3, PROPUESTA).
- Abrir un `.gaf` pasado en la línea de órdenes: hoy `gafira` no lee archivos de sus argumentos, así que la entrada no declara `%F` ni `MimeType=` (ver Riesgos).
- `xtask gen-brand` (03 §5.2) y el registro de licencias de `assets/` para estos iconos (son de la marca propia).

## Dependencias

- Brand: 03 §5.3 (composición y tamaños), DIS-13, manual de marca §7.12 y §12.4; `assets/gafira-logo-symbol.svg` y `docs/manuals/marca/assets/gafira-gaf-icon.svg`.
- `xtask` depende solo de `resvg` 0.45 sin funciones por defecto (sin texto ni fuentes del sistema), que ya estaba en el árbol por `egui_extras` con las mismas funciones; `Cargo.lock` gana solo el paquete `xtask`. No es un crate de dominio: sin fila V5 ni `clippy.toml`.
- `gafira-ui` usa `eframe::icon_data::from_png_bytes` (ya en `eframe`).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S46-01 | DIS-13: generar dos veces da los mismos bytes. | `dis_13_generation_is_deterministic`. |
| S46-02 | DIS-13: los archivos versionados son la salida del generador. | `dis_13_tracked_icons_match_the_generator` y `cargo xtask gen-icons --check`; un byte añadido a un PNG hace fallar `--check` con su ruta. |
| S46-03 | Los tamaños de Linux existen para los dos iconos. | `dis_13_pngs_have_the_hicolor_sizes`: 8 PNG por icono con su tamaño exacto y el SVG escalable; 18 archivos. |
| S46-04 | Composición de 03 §5.3. | `tile_has_rounded_canvas_corners_and_the_symbol_at_62_percent`: esquina transparente, borde superior `#FAFAF7` opaco, los píxeles del símbolo dentro de 97..=415 de 512 y ocupando más de 280 px. |
| S46-05 | El generador no deforma el símbolo. | `tile_rejects_a_symbol_it_cannot_copy_faithfully`, `style_fill_wins_over_the_attribute`. |
| S46-06 | Icono de ventana. | `app_icon::tests::window_icon_decodes_to_256_rgba`: 256 × 256 RGBA con el borde `#FAFAF7`; `shell::run` lo pone con `with_icon` y `with_app_id("gafira")`. |
| S46-07 | Escritorio Linux. | `desktop-file-validate packaging/linux/gafira.desktop` sin errores; `gafira-mime.xml` es XML válido; `sh -n install.sh` limpio. |

## TDD

El generador y el icono de ventana no son lógica crítica (AGENTS.md); sus pruebas se escribieron con el código. S46-02 se comprobó en rojo a mano: con un byte añadido a `16x16/apps/gafira.png`, `cargo xtask gen-icons --check` falló nombrando ese archivo, y `cargo xtask gen-icons` lo restauró.

## Evidencia

- `cargo xtask gen-icons --check`: «18 icon files up to date in assets/app-icon/hicolor».
- `cargo test -p xtask --locked`: 6 tests, 0 fallos. `cargo test -p gafira-ui --locked app_icon`: 1 test, 0 fallos.
- `desktop-file-validate packaging/linux/gafira.desktop`: sin salida (válido).
- `install.sh` no se ejecutó en esta spec (instrucción del titular): solo `sh -n` y la sustitución de `Exec=` probada aparte con `sed`. Prueba manual pendiente: `cargo build --release --locked -p gafira`, `packaging/linux/install.sh`, buscar «Gafira» en el menú, abrirla y ver el icono en el dock; `packaging/linux/install.sh --uninstall`.
- `cargo fmt --all --check`: limpio.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: limpio.
- `cargo test --workspace --locked`: 3156 tests, 0 fallos.
- `cargo deny check bans licenses sources`: ok (nuevo miembro `xtask` y dependencia `resvg` directa).
- ARQ-07 (`gafira-cli` sin UI): ok. V5, V12, ARQ-107, V13 y V14 no cambian (ningún crate de dominio tocado). `gafira-testkit`, `gafira-ports`, `ci.yml` y el manual técnico no cambiaron.
- `python3 tools/sync_agents.py --check`: 0.

## Riesgos y decisiones abiertas

- El radio de la baldosa (3/16 del lado) no lo fija 03 §5.3; lo decide el titular. Cambiarlo es una constante de `xtask/src/icons.rs` y volver a generar.
- La legibilidad del símbolo a 16 px no se ha revisado a ojo (manual §12.4 lo pide antes del lanzamiento); la versión simplificada para 16–32 px queda pendiente.
- Determinismo entre plataformas: los bytes se comprueban en Linux x86_64; resvg/tiny-skia con SIMD podrían dar otro redondeo en otra arquitectura. Si CI en Windows o macOS falla `dis_13_tracked_icons_match_the_generator`, se limita esa prueba a Linux.
- La entrada no asocia `.gaf` con Gafira (`MimeType=`, `Exec=gafira %F`) hasta que el binario abra el archivo que recibe; el tipo MIME y su icono sí se instalan.
- Seguimiento: `gafira.ico` y `gafira.icns` desde el mismo generador; icono de la ventana en Windows y macOS por empaquetado; abrir archivos desde la línea de órdenes.
