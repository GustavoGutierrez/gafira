# 029 — Imágenes en el lienzo

> **Estado:** done — 2026-11-16 — S29-01..S29-04 cubiertos por `gafira-doc` (`image::tests` 1, `validate::tests` 1, contratos ARQ-134 ampliados) y `gafira-ui` (`canvas::tests` 1, `editor::tests` 1 con el almacén real); 2853 tests.

## Objetivo

Importar una imagen y ponerla en la página (01 RF-19, en parte; 02 §3.1 `ImageNode`; 03 §2.3 Biblioteca): el archivo entra embebido en el almacén de contenidos (spec 026), el documento gana un elemento de imagen que nombra ese recurso por su hash, y el lienzo lo dibuja.

## Alcance / no alcance

Alcance:

- `gafira-doc`: primer corte de `ImageNode { asset }` y `NodeKind::Image`; P7 comprueba que la imagen nombra una entrada de la tabla de recursos (`ValidationError::DanglingImage`); `NodeVisitor::visit_image`; `Document::from_parts_with_assets`.
- `gafira-edit`: `InsertNodes` valida los nodos nuevos con la tabla de recursos del documento (sin ella, una imagen nueva nunca era válida).
- `gafira-ui`: Biblioteca → «Importar imagen…» (PNG y JPEG, diálogo del sistema) importa con `AssetService::import` e inserta la imagen centrada, al 60 % de la página como máximo y sin agrandarla; el lienzo la dibuja desde el almacén (escalada a 2048 px como máximo para la textura) con la opacidad del elemento; se mueve, redimensiona, ordena y borra como cualquier elemento; Capas la lista como «Imagen».

No alcance:

- Recorte, ajustes de imagen (RF-20), Zona de edición, quitar fondo.
- WebP, GIF, TIFF, BMP, QOI, SVG y HEIC (resto de RF-19); medios vinculados (Fase 2).
- Arrastrar y soltar archivos al lienzo; la Biblioteca como colección de archivos propios (RF-18).
- Importar y colocar en un solo paso de deshacer: son dos entradas (el recurso y el elemento).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S29-01 | Forma de cable. | `{"asset": "<hash hex>"}` va y vuelve. |
| S29-02 | Colocación. | Encaja en el 60 % de la página manteniendo la proporción y nunca se agranda. |
| S29-03 | De punta a punta. | Importar con el almacén real, insertar, decodificar la textura (40×20) y deshacer. |
| S29-04 | Validación. | Una imagen sin su recurso en la tabla no valida; con él, sí. |
