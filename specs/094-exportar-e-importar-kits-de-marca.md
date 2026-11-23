# 094 — Exportar e importar kits de marca (.gafkit)

> **Estado:** in-progress — 2026-11-23 — implementada y probada en Linux (D-125); pendiente de la prueba del titular y de probar el instalador de Windows y macOS en sus sistemas.

## Objetivo

Una persona comparte un kit de marca completo con otra, o lo lleva a otro equipo: lo exporta a un solo archivo `.gafkit` con todo lo que necesita (colores, estilos de texto, logos, fotos, gráficos, iconos, fuentes subidas y plantillas de la marca) y, en otro Gafira, lo importa y queda listo para usar.

## Alcance / no alcance

Alcance:

- **Formato `.gafkit`**: un ZIP con `kit.json` (formato, versión y el kit con rutas relativas) y las carpetas `logos/`, `photos/`, `graphics/`, `icons/`, `fonts/` y `templates/`.
- **Exportar**: desde la tarjeta del kit («···» → Exportar) y desde la página del kit; diálogo Guardar con `<nombre>.gafkit`.
- **Importar**: «Importar kit» en la pantalla Marca, y abrir un `.gafkit` con doble clic o desde la línea de órdenes. Si ya hay un kit con el mismo nombre (sin distinguir mayúsculas), Gafira lo dice y la persona elige: reemplazarlo, importarlo como uno nuevo o cancelar.
- Las fuentes del kit importado quedan instaladas en Gafira como las subidas (spec 093).
- **Tipo de archivo del sistema**: `application/vnd.gafira.brandkit+zip`, extensión `.gafkit`, con su icono, en Linux (MIME, entrada de escritorio, `.deb`, AppImage, `install.sh`), Windows (instalador) y macOS (`Info.plist`).

No alcance:

- Sincronizar kits entre equipos o en la nube.
- Mezclar dos kits (el reemplazo es completo).

## Dependencias

- Spec 093 (D-124): el modelo de kits y `BrandStore`.
- Spec 046 (DIS-13) y spec 090: generación de iconos (`cargo xtask gen-icons`) y empaquetado.

## Diseño

- `kit.json`: `{"format": "gafira-brandkit", "version": 1, "kit": {...}}`, con el kit en el mismo JSON que `kits.json`, pero con las rutas relativas al ZIP (`logos/logo.svg`).
- Al importar se leen solo entradas con nombre seguro (sin `..`, rutas absolutas ni carpetas desconocidas), con límites de número de entradas y de tamaño descomprimido. Los archivos se copian a la carpeta de un kit con id nuevo, o al del kit reemplazado, cuyos archivos anteriores se borran.
- El puerto `BrandStore` gana `export_kit`, `peek_kit` (nombre del kit de un `.gafkit`, para avisar del choque) e `import_kit` (como nuevo o reemplazando un id).
- Icono: `docs/manuals/marca/assets/gafira-gafkit-icon.svg`, derivado del icono `.gaf` (7.12): el mismo pliego con la esquina en coral, el símbolo, una paleta de cuatro colores y la barra con `.gafkit`.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S94-01 | Exportar. | El `.gafkit` tiene `kit.json` y todos los archivos del kit, incluidas fuentes y plantillas. |
| S94-02 | Importar. | Un `.gafkit` exportado e importado da un kit igual (nombre, paletas, estilos y archivos) con id libre; sus fuentes quedan instaladas. |
| S94-03 | Reemplazar. | Importar reemplazando un kit conserva su id, cambia su contenido y borra sus archivos anteriores. |
| S94-04 | Choque de nombre. | Si ya existe un kit con el mismo nombre, se pregunta: reemplazar, importar como nuevo o cancelar. |
| S94-05 | Seguridad. | Entradas con `..`, absolutas o fuera de las carpetas del kit se ignoran; un archivo que no es `.gafkit` da un error claro. |
| S94-06 | Tipo de archivo. | `.gafkit` se registra con su icono en Linux, Windows y macOS; abrir uno lleva a importarlo. |

## TDD

1. Ida y vuelta exportar → importar (S94-01, S94-02).
2. Reemplazar (S94-03).
3. Entradas peligrosas y archivo que no es un kit (S94-05).
4. `peek_kit` da el nombre (S94-04).

## Evidencia

- `apps/gafira`: `s94_01_02_a_kit_exported_and_imported_comes_back_whole` (nombre, paletas, estilos, logo, fuente instalada y plantilla con la etiqueta del kit nuevo), `s94_03_importing_over_a_kit_replaces_it`, `s94_05_unsafe_entries_are_ignored_and_other_files_refused`.
- `xtask`: los iconos de `.gafkit` (hicolor, `.ico`, `.icns`) generados y comprobados con los otros (`dis_13_*`, `s90_02_*`).
- La CI de publicación comprueba en Windows el tipo `.gafkit` y su icono tras instalar, y que se quita al desinstalar.
- Xephyr: abrir `Gafira Gynaika.gafkit` lleva a Marca y pregunta, porque ya hay un «Gynaika»; «Importar como nuevo» crea «Gynaika (2)» con sus 19 fuentes, logo y fotos; «Reemplazar» deja el mismo kit y el mismo número de kits. El diálogo Guardar de «Exportar kit» se abre por el portal del sistema y no se ve en Xephyr: la exportación se prueba con las pruebas unitarias.

## Riesgos y decisiones abiertas

- Un kit nuevo tiene id local (`kit-N`); el choque se detecta por nombre, no por id.
