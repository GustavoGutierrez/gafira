# 090 — Instalador de Windows y publicación de binarios

> **Estado:** in-progress — 2026-11-22 — implementada; pendiente de la primera publicación en Actions y de la prueba del titular en Windows.

## Objetivo

Cada versión publicada lleva sus binarios para Linux y Windows, con un instalador de Windows que registra los archivos `.gaf` y su icono, gastando los minutos de Actions imprescindibles.

## Alcance / no alcance

Alcance: ejecutable de Windows con icono y datos, instalador NSIS y `.zip`, iconos `.ico` e `.icns`, flujo de publicación, recorte de CI y paquete de macOS a petición.

No alcance: firma de código y notarización, `.rpm`, AppStream, actualizaciones automáticas, abrir en macOS un `.gaf` con doble clic (llega como evento del sistema, no como argumento).

## Dependencias

Specs 046 (iconos) y 078 (paquetes de Linux); ADR 0003; D-121 (esta spec). Dependencia de build nueva: `winresource` (MIT).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S90-01 | Ejecutable. | Icono y datos de versión; sin consola en release; con el runtime de C dentro; `--version` dice la versión. |
| S90-02 | Iconos. | `cargo xtask gen-icons` genera los `.ico` y `.icns` de la aplicación y del documento; `--check` los comprueba. |
| S90-03 | Instalador. | Español e inglés; accesos directos; «Aplicaciones instaladas»; silencioso con `/S`; desinstala solo lo suyo y deja proyectos y ajustes. |
| S90-04 | `.gaf`. | El instalador registra la extensión, el tipo MIME, el icono y la orden de abrir; al desinstalar, los quita si siguen siendo de Gafira. |
| S90-05 | Publicación. | Una etiqueta `v*` igual a la versión compila Linux y Windows, comprueba los paquetes y los publica con `SHA256SUMS` y las notas de `CHANGELOG.md`; con guion en la versión, como preliminar. |
| S90-06 | Coste. | Sin pruebas repetidas en la publicación; CI corre una vez por PR a `main`; macOS solo a petición. |

## TDD

`xtask`: `s90_02_the_windows_and_macos_icons`. `gafira-edit`: leyes de `DuplicatePage` y `ReorderPage` (V14).

## Evidencia

- Compilado con mingw e instalado con Wine 11: archivos, registro de `.gaf` con su icono y orden, `--version`, la ventana de Inicio y la desinstalación completa.
- El instalador de Actions se compila con MSVC y el propio flujo lo instala, comprueba el registro y lo desinstala.
- Ensayo de `release.yml` en `develop` (ejecución 45079357708): Linux y Windows en verde, con el instalador instalado, el registro comprobado y desinstalado en Windows real.
- `cargo deny check` completo, con avisos: `zip` 9.8 y `url` 2.5.9 corrigen sus vulnerabilidades; cinco crates sin mantenimiento y sin versión a la que pasar (`paste`, `ttf-parser`, `zbus_names`, `zvariant`, `zvariant_derive`) quedan exceptuados en `deny.toml` con la aprobación del titular.
- Suite completa: 3640 pruebas, 0 fallos. CI compila sin información de depuración para no llenar el disco del runner.
- V14 y `forbid(unsafe_code)` de `gafira-photos`, que fallaban desde antes, corregidos. La ley 4 descubrió que `DuplicatePage::touches` no declaraba los nodos de la copia; corregido.

## Riesgos y decisiones abiertas

- Sin firma, Windows y macOS avisan de editor desconocido.
- El paquete de macOS no se ha probado.
- Las pruebas de Windows y macOS ya no corren en cada PR; se lanzan a mano (`workflow_dispatch` de CI).
