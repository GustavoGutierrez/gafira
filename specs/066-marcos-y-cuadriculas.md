# 066 — Marcos y Foto collage: una foto recortada por una forma

> **Estado:** in-progress — 2026-11-21 — Fase 1 hecha; de la Fase 2, reencuadre con zoom y giro (S66-07), soltar una imagen del lienzo en un marco (S66-06) e imagen de muestra (S66-12); de la Fase 3, formas desde SVG y PDF (S66-10, crate `gafira-io-vector`). Contenido dentro del marco (D-107); la rejilla se llama «Foto collage» (D-106). Pendiente: prueba del titular y Foto collage (S66-09).

## Objetivo

Que una persona pueda colocar una foto dentro de una forma —un cuadrado, un círculo, un corazón, una estrella, el contorno de un logotipo— y que la foto se recorte a esa forma, se pueda reencuadrar dentro de ella y se pueda reemplazar sin perder la forma. Que la misma pieza se pueda crear de una vez en rejilla (varias celdas) y que la forma pueda venir de un vector: un SVG o, en Fase 2, una página de PDF.

## Origen

- **Producto:** FD-18 «Marcos con recorte por forma» (04 §5) y 02 §3.2, que ya fijan `FrameNode`, `clip_shape`, `content`, `content_offset`, `content_scale`, `fit` y el comando `SetFrameContent`. Esta spec los implementa; no los reinventa.
- **Referencia de comportamiento:** observación del editor de Canva con REA sobre CDP (`list_browser_targets`, `inspect_web_page`, árbol de accesibilidad): Elementos agrupa **Marcos** y **Cuadrículas** como contenedores de forma; al soltar una foto encima, la foto encaja recortada por la forma y se reencuadra dentro sin mover el contenedor. Se documenta el comportamiento, no el código: la implementación es propia y usa las piezas que ya existen en Gafira (forma vectorial + imagen + recorte al renderizar).
- **Licencia:** no se copia código, nombres internos ni recursos de Canva. Las formas de la biblioteca son propias y con licencia registrada (ARQ-75).

## Alcance / no alcance

**Fase 1:** `FrameNode` en el documento con validación y compatibilidad aditiva; el encaje y el recorte en el render, iguales en lienzo y exportación; soltar, reemplazar y vaciar contenido; la categoría «Marcos» en Elementos con clic y arrastre (D-105).

**Fase 2:** reencuadrar (S66-07); «Foto collage» (S66-09, D-106), la rejilla de marcos; miniaturas con la imagen de muestra (S66-12).

**Fase 3:** la forma desde un SVG (S66-10) —`gafira-io-svg` aún no existe: hay que crearlo con un lector de SVG de licencia permisiva— y, tras la importación de PDF, desde un PDF.

**No alcance:** importar PDF (Fase 2, depende de `gafira-io-pdf`); máscaras pintadas o por luminancia (ya son `Mask`, 02 §3.2); marcos animados; recorte no destructivo de la imagen original (la imagen nunca se modifica, FD-31); booleanas entre formas (D-80, Fase 2).

## Dependencias

- 02 §3.1 y §3.2 (`NodeKind::Frame`, `Placement`, arena y comandos), 04 FD-18 y FD-17, D-35 (grupos en espacio de página), D-68 (formato `.gaf`), D-92 (recorte al renderizar).
- Crates: `gafira-doc` (nodo y validación), `gafira-edit` (`SetFrameContent`), `gafira-geom` (afín y caja), `gafira-pathops` (contorno del trazo y unión de rutas), `gafira-render` (grafo), `gafira-render-cpu` (píxeles), `gafira-io-image` y `gafira-io-pdf` (exportación), `gafira-ui` (panel, Inspector, lienzo), `gafira-i18n` (es/en).
- Fase 2: `gafira-io-svg` para leer el SVG y `gafira-io-pdf` (hayro 0.9.0, Apache-2.0 OR MIT, verificado en 04 §13) para leer el PDF.

## Modelo de datos

Cambio aditivo del esquema 1: un nodo nuevo, `NodeKind::Frame(FrameNode)`, sin tocar los nodos existentes. Un documento anterior no puede contenerlo y no se ve afectado; un documento con marcos no abre en una versión anterior (misma regla que D-101 y D-103).

```rust
pub struct FrameNode {
    /// La forma que recorta: la misma geometría de ShapeNode, en la caja del marco.
    pub clip_shape: ShapeGeometry,
    /// El contorno dibujado sobre el borde recortado (04 §8).
    pub stroke: Option<ShapeStroke>,
    /// El contenido; `None` es un marco vacío.
    pub content: Option<FrameContent>,
    /// Cómo encaja la imagen en la forma.
    pub fit: Fit,
    /// Desplazamiento del contenido, en fracción de la caja del marco.
    pub content_offset: (f64, f64),
    /// Multiplicador sobre la escala que calcula `fit`; `> 0`.
    pub content_scale: f64,
}

pub struct FrameContent {
    pub asset: AssetRef,
    /// Tamaño intrínseco de la imagen, para calcular el encaje sin abrir el archivo.
    pub natural: (u32, u32),
}

pub enum Fit { Fill, Fit, Stretch, None }
```

`FrameContent` guarda el recurso por hash (D-68), no un `NodeId`: el contenido de un marco no es una capa independiente, no se selecciona con el marco ni aparece suelto en Capas, y así reemplazarlo es un solo comando atómico. Esto **precisa** el `content: Option<NodeId>` de 02 §3.2 y lo confirmó el titular (D-107).

Invariantes que valida `Document::validate`: `content_scale` finito y `> 0`; `content_offset` finito; `asset` existe en la tabla (P7); `clip_shape` no vacío; `stroke` con grosor en rango.

## Reglas de encaje

Sea la caja del marco `W × H` en la página y la imagen `w × h` naturales. `fit` fija la escala base `s0` y el desplazamiento base:

| `fit` | `s0` | Resultado |
| --- | --- | --- |
| `Fill` | `max(W/w, H/h)` | Cubre la caja; lo que sobra se recorta |
| `Fit` | `min(W/w, H/h)` | Cabe entera; el resto queda transparente |
| `Stretch` | `sx = W/w`, `sy = H/h` | Deforma hasta llenar |
| `None` | `1` | Tamaño natural, centrada |

Escala final `s = s0 · content_scale`. El centro de la imagen se coloca en el centro de la caja más `content_offset · (W, H)`.

- Con `Fill`, el desplazamiento se **acota** para que la imagen siga cubriendo la caja: `|offset_x| ≤ (s·w − W) / (2W)` y lo análogo en `y`. Sin la cota, arrastrar dejaría huecos y el marco parecería roto.
- Con `Fit`, `Stretch` y `None` el desplazamiento es libre dentro de la caja.
- Reencuadrar nunca cambia la caja ni la forma del marco, solo `content_offset` y `content_scale`.
- Un marco vacío se dibuja como la forma con su relleno (si lo tiene) y un aviso visual en el lienzo; no se exporta como un hueco invisible salvo la opción «Incluir marcos vacíos» (FD-18 c).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S66-01 | Modelo y validación. | `FrameNode` con `clip_shape`, `stroke`, `content`, `fit`, `content_offset`, `content_scale`; ida y vuelta por JSON sin pérdida; `content_scale ≤ 0`, offset no finito o `asset` inexistente no validan; un documento sin marcos conserva sus bytes. |
| S66-02 | Comando de contenido. | `SetFrameContent` asigna, reemplaza y quita el contenido; un solo paso de deshacer por operación; reemplazar conserva forma, caja y encaje; quitar deja el marco vacío. |
| S66-03 | Encaje. | `Fill`, `Fit`, `Stretch` y `None` cumplen la tabla; con `Fill` el offset se acota y nunca deja hueco; el cálculo es determinista para las mismas entradas. |
| S66-04 | Recorte en el render. | El grafo emite el recorte de la forma y dibuja la imagen dentro; fuera de la forma el píxel es transparente y dentro se ve la imagen con su transformación. |
| S66-05 | Lienzo y exportación iguales. | El lienzo dibuja la misma imagen que el grafo de exportación; PNG y PDF muestran el mismo recorte y el mismo encuadre. |
| S66-06 | Soltar una imagen. | Arrastrar una imagen de Biblioteca sobre un marco le asigna el contenido y aplica `Fill`; soltarla sobre un marco ocupado pregunta antes de reemplazar; un marco vacío acepta el soltado en toda su caja. |
| S66-07 | Reencuadrar. | Doble clic entra en modo reencuadre: arrastrar desplaza el contenido, rueda o `+`/`-` con `Ctrl` escalan, `Esc` sale; todo el gesto es un solo paso de deshacer. |
| S66-08 | Panel y biblioteca. | Elementos → «Marcos» con las formas de la biblioteca (cuadrado, redondeado, círculo, corazón, estrella, rombo, arco, película); al pulsar se inserta centrado y seleccionado, con el 25 % de la página (D-71); también se arrastra al lienzo (D-105). |
| S66-09 | Rejilla. | Una acción inserta `filas × columnas` marcos con separación editable, agrupados; redimensionar el grupo escala las celdas; las celdas conservan su forma. |
| S66-10 | Forma desde un vector. | Importar un SVG propio crea un marco con su contorno como `clip_shape`; la forma conserva el relleno no cero y se puede reemplazar por otra sin perder el contenido. |
| S66-11 | Accesibilidad e idioma. | Cada botón y campo tiene nombre accesible; textos en es/en por catálogo; el marco vacío anuncia «Marco vacío»; el contenido aporta su texto alternativo a la exportación accesible (FD-55). |
| S66-12 | Imagen de muestra. | La biblioteca de marcos y los tests visuales usan `assets/samples/frame-sample.png`, registrado en `assets/LICENSES.toml`; la miniatura de cada forma lo recorta con la misma regla que el lienzo. |

## Rejilla y nombre

En Gafira **«Cuadrícula» ya es la rejilla de fondo** (FD-07). La rejilla de fotos se llama **«Foto collage»** («Photo collage», D-106), la categoría de Elementos que sustituye a «Líneas».

Fase 1: la rejilla es un `GroupNode` de marcos ya colocados; redimensionar escala, no refluye. Fase 2 (decisión abierta): un `GridNode` persistente con filas, columnas, separación y reflujo al redimensionar.

## Imagen de muestra

La biblioteca de marcos necesita una foto de ejemplo para que cada forma se vea como lo que es: un recorte. Se usa una sola imagen propia, `assets/samples/frame-sample.png`, proporcionada por el titular el 21 de noviembre de 2026 (paisaje plano: cielo, sol coral, nubes y colinas).

Está optimizada para el repositorio y no se toca al implementar:

- Original: PNG RGB 1672 × 941, 1 133 343 B, 9 572 colores (ruido de compresión de ±1 nivel sobre los colores planos).
- Publicada: PNG de 64 colores, 1672 × 941, 28 777 B (−97,5 %), paleta construida fusionando colores a distancia ≤ 12 y sin tramado; error medio 0,5/255 y máximo 42/255 solo en los bordes.
- Se mantiene la resolución nativa: no se remuestrea, así el borde del sol y de las nubes conserva su antialias.
- Registrada en `assets/LICENSES.toml` (ARQ-75) con `license = "Propia"`, `modificado = "true"` y el comando de optimización.

Usos: miniatura de cada forma en Elementos → Marcos; contenido de ejemplo al insertar un marco de demostración; fixture de los tests visuales de recorte (S66-05 y S66-12). No se usa como contenido por defecto de un marco real: un marco nuevo nace vacío.

## Forma desde PDF o vector

- **SVG (Fase 3):** un crate nuevo `gafira-io-svg` convertirá rutas a `ShapePath`; el contorno elegido se normaliza a la caja unitaria y se guarda como `clip_shape`. Si el SVG trae varias rutas, se ofrece unirlas (`gafira-pathops`) o quedarse con una.
- **PDF (Fase 2):** la importación de PDF está en Fase 2 en 04 §13. Cuando exista, se extraen las rutas vectoriales de una página, se elige una y se crea el marco igual que con el SVG. Un PDF escaneado no aporta vectores: se avisa y se ofrece la imagen de la página como contenido, no como forma.
- En ambos casos el marco resultante es editable como cualquier otro: se le puede cambiar el contenido y el encaje, y la forma no depende del archivo de origen.

## TDD

Casos que deben fallar primero (lógica crítica: modelo, validación, geometría y recorte):

1. `gafira-doc/tests/frame.rs`: `s66_01_round_trip`, `s66_01_validation_rejects_bad_content`, `s66_01_a_document_without_frames_keeps_its_bytes`.
2. `gafira-edit`: `s66_02_set_replace_and_remove_are_one_undo_each`.
3. `gafira-geom` o `gafira-render`: `s66_03_fill_covers`, `s66_03_fit_contains`, `s66_03_stretch_fills`, `s66_03_none_keeps_natural_size`, `s66_03_fill_offset_is_clamped`.
4. `gafira-render`: `s66_04_a_frame_emits_a_clip_and_an_image`.
5. `gafira-render-cpu/tests/frames.rs`: `s66_05_outside_the_shape_is_transparent`, `s66_05_inside_shows_the_image`.
6. `gafira-io-image` / `gafira-io-pdf`: `s66_05_png_and_pdf_agree_on_the_crop`.
7. `gafira-ui`: `s66_06_dropping_an_image_fills_the_frame`, `s66_09_a_grid_has_rows_columns_and_gaps`, `s66_12_the_frame_thumbnail_uses_the_sample`.
8. `gafira-io-svg`: `s66_10_an_svg_path_becomes_a_frame_shape`.

## Implementación de la Fase 1

- `gafira-doc`: `NodeKind::Frame(FrameNode)`, `FrameContent`, `Fit`, `content_transform` (encaje y cota de `Fill`), validación `InvalidFrame` (números y P7).
- `gafira-edit`: `Property::FrameContent` y `Property::FrameFit` en `SetProperty` (un paso de deshacer cada uno, cumple S66-02 sin un comando aparte); `Stroke` y `CornerRadius` valen también para marcos.
- Contrato del render: `RenderOp::ClipPath { path, transform }` (aditivo; recorta la capa abierta por la cobertura de la ruta). El grafo emite `DrawImage` + `ClipPath` + contorno; el backend CPU lo implementa; la exportación decodifica las fotos de los marcos.
- `gafira-ui`: Elementos → Marcos (ocho formas, clic y arrastre, D-105), el marco vacío en gris en el lienzo (no se exporta), la foto recortada como textura con tiny-skia, Inspector «Foto» («Poner foto…», «Cambiar foto…», «Quitar foto») y «Encaje».
- Tests: `gafira-doc/tests/frame.rs` (S66-01, S66-03), `s66_02_*` (gafira-edit), `s66_04_*` (graph), `gafira-render-cpu/tests/frames.rs` (S66-05), `s66_05_the_canvas_cuts_the_photo_to_the_shape` y `s66_08_*` (gafira-ui).

## Implementación posterior (petición del titular, con capturas)

- **Soltar una imagen del lienzo en un marco (S66-06):** al arrastrar una sola imagen sobre un marco, el contorno del marco se ilumina (`primary.bg`), el marco muestra la foto recortada y la imagen arrastrada se atenúa; al soltar se deshace el movimiento y un solo paso pone la foto en el marco (tamaño natural leído de la cabecera) y quita la imagen.
- **Modo recorte (S66-07):** doble clic en un marco con foto. La foto entera se ve atenuada alrededor y el marco iluminado; arrastrar mueve la foto (`FrameNode::moved`, con la cota de «Rellenar»); el Inspector pasa a «Recortar» con cerrar (×), «Zoom» 100–400 %, «Restablecer», ayuda y «Listo». Esc, × o un clic fuera terminan. Toda la sesión es un gesto: un paso de deshacer (`Property::FrameCrop`). «Girar» (−180° a 180°) gira la foto sobre su centro dentro del marco: `FrameNode.content_rotation_deg` (aditivo, solo se escribe si no es 0); con «Rellenar» la foto girada crece lo justo para seguir cubriendo el marco y el arrastre se acota para no destapar ninguna esquina (`s66_07_a_turned_photo_keeps_covering_the_box`). El arrastre no tiene en cuenta el giro del propio marco.
- **Suprimir:** sobre un marco con foto quita la foto y deja el marco vacío y seleccionado; sobre un marco vacío (o cualquier otro elemento) lo borra. Un paso de deshacer (`s66_delete_empties_a_full_frame_and_removes_an_empty_one`).
- **Forma desde SVG o PDF (S66-10):** crate adaptador `gafira-io-vector` (usvg y hayro-syntax, ya en el árbol, licencias MIT/Apache-2.0). Reúne los trazados rellenos (o, en un PDF sin rellenos, los de recorte `W n`, como el de un vectorizado que recorta una imagen), los lleva a la caja unidad y guarda sus proporciones; si pasan de `ShapePath::MAX_COMMANDS`, se aplanan y simplifican (Ramer–Douglas–Peucker) hasta caber. Se importa con «Importar forma (SVG o PDF)…» en Elementos → Marcos, o soltando/pegando el archivo (spec 067). La validación del documento revisa también la ruta de un marco. Tests: `s66_10_*` (gafira-io-vector y gafira-ui).
- **Imagen de muestra (S66-12):** un marco vacío muestra `assets/samples/frame-sample.png` recortada por su forma en el lienzo y en las baldosas de Elementos → Marcos; no se exporta (FD-18 c).
- Tests: `s66_07_*` (gafira-doc), `s66_02_*` con `FrameCrop` (gafira-edit), `s66_06_s66_07_an_image_into_a_frame_then_cropped` y `s66_05_the_canvas_cuts_the_photo_to_the_shape` con la muestra (gafira-ui).

## Evidencia esperada

- `cargo test -p gafira-doc -p gafira-edit -p gafira-render -p gafira-render-cpu -p gafira-ui --locked` en verde.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` y `cargo fmt --check`.
- Exportación PNG y PDF de una página con tres marcos (círculo, corazón y estrella) revisada a ojo sobre fondo claro y oscuro.
- Captura del panel Elementos → Marcos y del modo reencuadre, con la imagen de muestra recortada por cada forma.
- `assets/samples/frame-sample.png` presente, con su entrada en `assets/LICENSES.toml` y el hash BLAKE3 del archivo publicado.
- Prueba manual de soltar, reemplazar y vaciar, y de una rejilla de 2 × 3 redimensionada.
- `python3 tools/sync_agents.py --check` si se toca `.agents/`.

## Riesgos y decisiones abiertas

- **Contrato del render.** `RenderOp` no tiene recorte. Hay dos caminos: añadir `PushClip { path, transform }` / `PopClip` (aditivo, conserva vectores y lo aprovechan PDF y SVG), o renderizar la imagen en una capa y aplicarle `ApplyMask` con la forma rasterizada (no toca el contrato, cuesta una máscara por marco y pierde nitidez de borde). Se propone el primero; afecta a `gafira-render-cpu` y a los exportadores.
- **Contenido como recurso o como nodo hijo.** Resuelto: dentro del marco (D-107).
- **Nombre de la rejilla.** Resuelto: «Foto collage» (D-106).
- **Rendimiento.** Un recorte por marco en una página con muchos marcos puede multiplicar el trabajo; se medirá con 50 marcos antes de cerrar (CAL-02).
- **PDF.** La forma desde PDF no se puede entregar antes de la importación de PDF (Fase 2); la spec no la promete en Fase 1.
- **Imágenes muy grandes.** El encaje usa el tamaño natural guardado; si no coincide con el archivo, se recalcula al abrir y se avisa (misma regla que FD-31 para metadatos).
