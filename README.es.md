<p align="center">
  <!--<img src="docs/manuals/marca/assets/gafira-logo-primary.png#gh-light-mode-only" alt="Gafira" width="240"> #gh-dark-mode-only -->
  <img src="docs/manuals/marca/assets/gafira-logo-dark-background.png" alt="Gafira" width="240">
</p>

<p align="center">
  <a href="https://github.com/GustavoGutierrez/gafira/releases"><img src="https://img.shields.io/github/v/release/GustavoGutierrez/gafira?include_prereleases&sort=semver&label=versi%C3%B3n&color=3FBFA9" alt="Última versión"></a>
  <a href="https://github.com/GustavoGutierrez/gafira/releases"><img src="https://img.shields.io/github/downloads/GustavoGutierrez/gafira/total?label=descargas&color=3FBFA9" alt="Descargas"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/licencia-GPL--3.0--or--later-18243B" alt="Licencia: GPL-3.0-or-later"></a>
  <a href="rust-toolchain.toml"><img src="https://img.shields.io/badge/Rust-1.93.1-18243B?logo=rust" alt="Rust 1.93.1"></a>
  <br>
  <img src="https://img.shields.io/badge/plataformas-Windows%20%7C%20macOS%20%7C%20Linux-18243B" alt="Plataformas: Windows, macOS, Linux">
  <img src="https://img.shields.io/badge/idiomas-Espa%C3%B1ol%20%7C%20English-18243B" alt="Idiomas: español, inglés">
  <a href="https://gafira.com/"><img src="https://img.shields.io/badge/web-gafira.com-FF6B57" alt="Web: gafira.com"></a>
  <a href="https://gafira.com/#support"><img src="https://img.shields.io/badge/ap%C3%B3yanos-paquetes%20de%20ilustraciones-FF6B57?logo=gumroad&logoColor=white" alt="Apoya a Gafira: paquetes completos de ilustraciones"></a>
</p>

<p align="center"><a href="README.md">English</a> · <b>Español</b></p>

# Gafira

**Haz visibles tus ideas. Bring your ideas to life.**

Gafira es un programa de diseño gráfico local para todas las personas y creadores de contenido, escrito en Rust (egui). Elige una plantilla, personalízala y compártela. Funciona en Windows, macOS y Linux, en español e inglés. Es software libre y de código abierto bajo la GPL, versión 3 o posterior.

Versión actual: `0.2.0` (ver `CHANGELOG.md`).

Web: <https://gafira.com> (español) · <https://gafira.com/en/> (inglés). Su código está en `website/`.

## Vista previa

<p align="center">
  <img src="assets/splash_screen.jpg" alt="Pantalla de inicio de Gafira" width="640">
</p>

## Qué hace

- **Empezar:** Inicio con búsqueda, diseños recientes y 75 plantillas por idioma (redes sociales, presentaciones, impresos, invitaciones, documentos, web y correo), o un diseño con tu propio tamaño.
- **Diseñar:** 215 formas en 17 secciones (líneas, polígonos, estrellas, flechas, diagramas, bocadillos, nubes, comillas), líneas que se conectan a las formas, texto con 33 familias tipográficas incluidas, texto curvo, texto dentro de formas y efectos de texto, dibujo a mano alzada con 16 pinceles, marcos y collage de fotos, tablas (importación de Excel y CSV), gráficos y códigos QR.
- **Color:** un mismo campo de color en todas partes, con cuentagotas y lupa, colores sólidos y degradados, colores del diseño y paletas predefinidas; fondos de página con color, degradado, patrón o imagen.
- **Imágenes:** ajustes no destructivos, filtros, sombras y recortes con formas; quitar el fondo, aumentar la resolución y el borrador mágico con modelos locales (CPU o GPU); fotos e ilustraciones libres desde el panel Elementos.
- **Organizar:** Capas con arrastrar y soltar, bloquear y ocultar; guías, ajuste magnético, reglas, alinear y distribuir; versiones dentro del archivo `.gaf` y recuperación tras un cierre inesperado.
- **Marca y paquetes:** kits de marca (logotipos, colores, estilos de texto, fuentes) que se comparten como `.gafkit`; paquetes de recursos (`.gafpack`) que se instalan desde Inicio y se desinstalan desde Ajustes o desde su panel.
- **Exportar:** PNG, JPEG, WebP, AVIF, SVG, PDF vectorial (CMYK, contraseña) y PowerPoint editable, en segundo plano.

## Descargar

Los instaladores de cada versión están en las [versiones de GitHub](https://github.com/GustavoGutierrez/gafira/releases) y en <https://gafira.com/#download>: instalador y `.zip` portable para Windows, `.dmg` para macOS (Apple Silicon, sin notarizar), `.deb` y AppImage para Linux.

## Apoya el proyecto

Gafira es gratis. Si quieres apoyar su desarrollo, puedes comprar uno de los [paquetes completos de ilustraciones](https://gafira.com/#support): te llevas todas sus ilustraciones y nos ayudas a seguir construyendo Gafira. También puedes contribuir con código, traducciones o reportes de errores (ver `CONTRIBUTING.md`).

## Requisitos

- Toolchain de Rust fijado en `rust-toolchain.toml` (`rustup` lo instala automáticamente).
- Opcional: `cargo-deny`, para comprobar las licencias de terceros con `deny.toml`.

## Compilar y comprobaciones

```
cargo build --workspace --locked
```

Cada commit debe pasar estas tres comprobaciones:

```
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

## Iconos e instalación en Linux

Los iconos de la aplicación y de los archivos `.gaf` se generan a partir de los SVG de la marca (`specs/done/046-application-icon.md`):

```
cargo xtask gen-icons            # escribe assets/app-icon/hicolor/
cargo xtask gen-icons --check    # falla si un icono versionado está desactualizado
```

Instalación por usuario en Linux (no compila nada; `--uninstall` la elimina):

```
cargo build --release --locked -p gafira
packaging/linux/install.sh [ruta/a/gafira]
```

## Paquetes y versiones

| Sistema | Comando | Resultado en `dist/` |
|---|---|---|
| Linux | `packaging/linux/build-packages.sh` | `.deb` y AppImage |
| Windows (o Linux con `mingw-w64` y `nsis`) | `packaging/windows/build.sh` | instalador y `.zip` portable |
| macOS | `packaging/macos/build.sh` | `.dmg`, sin notarizar |

Una etiqueta `v<versión>` en `main` compila Linux y Windows en `.github/workflows/release.yml` y los publica con las notas de `CHANGELOG.md` (spec 090). Ejecuta antes ese workflow a mano sobre `develop`: comprueba los paquetes y deja la caché que usa la etiqueta. macOS (runner `macos-15`) solo se compila cuando el workflow se ejecuta a mano sobre la etiqueta con `macos: true`; añade el `.dmg` a la misma versión. Después de publicar una versión, vuelve a compilar y desplegar la web para que sus enlaces de descarga apunten a la nueva.

## Estructura del repositorio

| Ruta | Contenido |
|---|---|
| `crates/` | Crates de biblioteca `gafira-*` (capas hexagonales verificadas en CI); la interfaz de escritorio es `gafira-ui` |
| `apps/` | Binarios: `gafira` (la aplicación de escritorio) y `gafira-cli` |
| `website/` | La página de gafira.com (Astro, estática, Cloudflare Workers); ver `website/README.md` |
| `specs/` | Specs numeradas (en español): activas en `specs/`, sin empezar en `specs/backlog/`, cerradas en `specs/done/` (índice y siguiente ID libre en `specs/README.md`). Documentos de referencia del producto `00-..07-*.md` (`00-decisiones.md` reúne las decisiones del titular); los cambios, el historial y las capacidades del flujo anterior están en `specs/done/legacy/` |
| `assets/` | Pantalla de inicio, tokens de diseño (`theme/gafira.tokens.json`, que carga `gafira-theme`), catálogos de la interfaz (`i18n/es.toml`, `i18n/en.toml`, compilados en `gafira-i18n`), fuentes y otros recursos de diseño (los logotipos están en `docs/manuals/marca/assets/`) |
| `xtask/` | Tareas del repositorio (`cargo xtask gen-icons`) |
| `packaging/` | Paquetes e integración con el escritorio: `linux/` (`.deb`, AppImage, entrada de escritorio, tipo MIME `.gaf`, `install.sh`), `windows/` (instalador NSIS, `.zip`), `macos/` (`.dmg`) |
| `.agents/` | Skills y el agente `manual-writer`, la fuente única para todos los entornos; `.claude/`, `.opencode/` y `.pi/` enlazan a ella (`python3 tools/sync_agents.py`) |

## Flujo de desarrollo

El agente (Claude Code u otro) implementa las specs directamente, siguiendo `AGENTS.md` y el skill `gafira-specs`:

- Las specs tienen un ID de tres dígitos y un banner de estado (`backlog`, `in-progress`, `done`); `specs/README.md` es el índice.
- TDD para la lógica crítica (modelo de documento, serde y validación, comandos de edición e historial, tiempo, color, geometría y operaciones con trazados, límites, lectura de formatos); pruebas de integración por función para el resto; suite completa y comprobaciones al cerrar la spec.
- El trabajo se hace en una rama corta creada desde `develop` y se fusiona en `develop` cuando pasa sus comprobaciones; el único PR es `develop` -> `main`, que se abre solo cuando el titular lo pide o en un punto digno de versión, y el titular lo revisa y lo fusiona (`docs/adr/0003-develop-integration-branch.md`, `docs/adr/0018-simple-spec-flow.md`).
- `cargo-mutants` es una herramienta opcional para la lógica crítica.

Primera configuración: genera el registro de skills, que no se versiona (`.atl/skill-registry.md`):

```
.agents/skills/skill-registry/refresh.sh
```

La metodología completa (reglas, TDD, comprobaciones, ramas) está en `AGENTS.md` (en inglés).

## Recursos

El audio de `assets/musics/` no se guarda en git (decisión D-10 del titular en `specs/00-decisiones.md`). Guarda ahí tus copias locales y no las incluyas en los commits.

## Licencia

Gafira es software libre: puedes redistribuirlo y modificarlo según los términos de la Licencia Pública General de GNU publicada por la Free Software Foundation, ya sea la versión 3 de la licencia o (a tu elección) cualquier versión posterior (`GPL-3.0-or-later`; ver `LICENSE`, el texto oficial en inglés). Se distribuye sin ninguna garantía.

El nombre, el logotipo y el icono de Gafira no están cubiertos por esa licencia; ver `TRADEMARKS.md`. Los paquetes de ilustraciones (`.gafpack`) llevan su propia licencia dentro de cada paquete. Los crates de terceros deben tener licencias permisivas (`deny.toml`).

¿Quieres ayudar? Lee `CONTRIBUTING.md` (en inglés).
