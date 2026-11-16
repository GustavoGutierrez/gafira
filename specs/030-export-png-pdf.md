# 030 — Exportar PNG y PDF

> **Estado:** in-progress — 2026-11-16 — rama `feat/030-export-png-pdf`.

## Objetivo

Que «Exportar» del Editor de diseño (03 §7.5) produzca un PNG de la página actual y un PDF del documento con lo que el lienzo muestra hoy: formas (rectángulo y elipse con relleno y opacidad) y textos (spec 027). El camino es el de 02: un `ExportService` en el engine (02 §5.4) que respeta la licencia (02 §4.14), elige el `Exporter` del `FormatRegistry` (02 §4.5) y le da un `PageRenderer` sobre `gafira-render`; los exportadores viven en adaptadores de capa 4 (`gafira-io-image`, `gafira-io-pdf`; ARQ-06).

## Alcance / no alcance

Alcance:

- `gafira-render`:
  - El grafo dibuja el `TextNode` como contornos de glifos (`gafira-text`: `layout_box` y `outline_text`, 02 §4.6 «El texto llega como contornos»), con un libro de fuentes (`FontBook`, familia → bytes) que inyecta quien compone; una familia desconocida usa la primera del libro.
  - En calidad de exportación, la `scale` de la petición escala la página (FD-55, «Escala»): la región está en píxeles de salida y el nivel sigue siendo el cero.
  - `GraphPageRenderers`: la fábrica de `PageRenderer` por instantánea del documento, sobre un backend que da quien compone.
- `gafira-ports::export`: el puerto `PageRenderers` (una instantánea del documento → un `PageRenderer`), para que el engine no vea `gafira-render` (ARQ-06).
- `gafira-io-image` (nuevo, capa 4): `PngExporter` (FD-55): escala de 0,5× a 4×, fondo blanco o transparente, sRGB 8 bits con el bloque `sRGB`, límite de 16 384 px por lado (D-71); un archivo por página del alcance.
- `gafira-io-pdf` (nuevo, capa 4): `PdfExporter` con krilla (FD-56): una página PDF por página del documento, del tamaño de la página en puntos, con la página rasterizada a la escala pedida (2× por defecto).
- `gafira-engine`: `ExportService::export(request, destino, cancel)`, síncrono: comprueba `LicenseService::can_export`, el documento abierto, el exportador del registro y su `validate` (FD-58); escribe a `*.tmp` por `FileSystem` y renombra al terminar (02 §6.4); ante un error no deja archivo final.
- `gafira-ui`: el botón coral «Exportar» abre un diálogo pequeño (módulo `export`): PNG de la página actual a 1× o 2×, o PDF de todas las páginas; luego el diálogo de guardar del sistema y el aviso «Tu diseño está listo para compartir» (`export.done`).
- `apps/gafira`: registra los dos exportadores y compone `GraphPageRenderers` con `CpuBackend` y las fuentes de `gafira-theme`.

No alcance:

- Imágenes: el documento todavía no tiene nodo de imagen en `develop` (spec 029 en paralelo). El enganche está en `RenderGraphBuilder`: un `visit_image` que registre el nodo y emita `RenderOp::DrawImage`.
- PDF vectorial y texto seleccionable (FD-56 (b)), preajustes Digital/Impresión, sangrado y marcas de corte (FD-05), contraseña, CMYK: el PDF de este corte es una imagen por página («Aplanar»).
- JPEG, WebP, GIF, SVG; «Selección» y «Rango» de FD-57 (el engine acepta cualquier lista de páginas); calidad y «Limitar tamaño» del PNG; vista previa y tamaño estimado del diálogo de 03 §7.5; «Mostrar en carpeta».
- La cola de exportación persistida, los estados y los reintentos de 02 §6.4: la exportación corre en el hilo de la UI de forma síncrona; el progreso y la cancelación pasan por los puertos pero la UI no los muestra.
- Tramos con estilos distintos: el texto se maqueta con el estilo del primer tramo (como el Inspector de la spec 027); tamaños por debajo de 1 pt o por encima de 1000 pt se acotan para `gafira-text`.

## Dependencias

- 02 §4.5 (`Exporter`, `ExportJob`, `PageRenderer`), §4.6, §5.4, §6.2, §6.4; ARQ-06, ARQ-42, ARQ-50.
- 04 FD-55, FD-56, FD-57, FD-58; 03 §7.5; D-71 (16 384 px por lado).
- Specs 010 (`LicenseService`, `NoLicensing`), 020 y 023 (formas, relleno y opacidad), 027 (`TextNode`).
- Crates nuevos: `png` 0.18 (ya en el workspace) y `krilla` 0.8.2 (MIT OR Apache-2.0, verificado en 02 §9.2 y FD-56).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S30-01 | Texto en el render. | Un texto con relleno de color pinta píxeles de ese color dentro de su caja y ninguno fuera; sin texto o sin fuentes no dibuja nada; el contenido entra en la clave del nodo. |
| S30-02 | Escala de exportación. | A 2× la página de 100×50 se lee como 200×100 y una forma en (10, 10, 20, 20) cubre (20, 20, 40, 40); una escala no finita o ≤ 0 es un error. |
| S30-03 | PNG. | Un PNG de la página tiene las dimensiones de la página por la escala, el color de una forma conocida en su sitio y blanco (o alfa 0 con fondo transparente) fuera; lleva el bloque `sRGB`; más de 16 384 px por lado es un error. |
| S30-04 | PDF. | Un documento de N páginas da un PDF que un lector analiza con N páginas del tamaño de cada página en puntos. |
| S30-05 | `ExportService`. | Exporta por el registro con el `PageRenderer` del puerto; un formato desconocido, la falta de documento o un aviso bloqueante de `validate` no escriben nada; el archivo final aparece solo al terminar, por `*.tmp` y `replace`, y un fallo del exportador no lo deja. |
| S30-06 | Licencia. | Con `can_export` falso la exportación se rechaza antes de abrir ningún archivo. |
| S30-07 | UI. | «Exportar» abre el diálogo; PNG a 1× y 2× y PDF construyen la petición correcta (formato, alcance y opciones); al terminar el aviso es `export.done`. |

## TDD

Lógica crítica: S30-02 (transformaciones y tamaños) y la conversión de píxeles del PNG; S30-05 y S30-06 (invariantes de escritura segura y de licencia) en `gafira-engine` con dobles. Deben fallar primero. S30-01, S30-03 y S30-04 con tests de integración sobre `CpuBackend`.

## Evidencia esperada

Tests nuevos en `gafira-render`, `gafira-io-image`, `gafira-io-pdf`, `gafira-engine` y `gafira-ui`; gates de `AGENTS.md` («Before calling something done»), `cargo deny check licenses bans sources` y los pasos de política de `ci.yml` en verde.
