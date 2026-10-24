# Especificación técnica para replicar un editor tipo Canva en Rust

Un editor tipo Canva se puede modelar como un **documento de diseño estructurado**, renderizado sobre un lienzo, con operaciones de edición, undo/redo, exportación y dos funciones de IA acotadas: eliminación de fondo y reescritura de texto. La arquitectura más robusta separa el **modelo del documento**, el **motor de renderizado**, la **capa de interacción** y los **servicios de exportación/IA**.

## Arquitectura general

Canva divide su herramienta de dibujo en capas: eventos de puntero, despacho de eventos, máquina de estados y capa de aplicación que distingue entre vista previa y confirmación del cambio. Ese patrón es directamente replicable en Rust.[^1]

```text
UI / Shell
 ├── Barra superior: archivo, deshacer, zoom, exportar
 ├── Panel izquierdo: plantillas, elementos, texto, imágenes, marcos
 ├── Lienzo central: documento, selección, guías, transformaciones
 ├── Barra contextual: propiedades del elemento seleccionado
 └── Panel derecho: capas, páginas, ajustes

Núcleo del editor
 ├── Document Store: páginas, capas, elementos, estilos
 ├── Command Stack: undo / redo
 ├── Selection Service: hit-testing, marquee, snapping
 ├── Transform Service: mover, escalar, rotar, alinear
 ├── Layout Engine: texto, ajuste, alineación, restricciones
 ├── Render Engine: raster / vector / GPU
 └── Export Service: PNG, JPG, PDF, SVG, MP4

Servicios
 ├── Asset Store: imágenes, íconos, fuentes, plantillas
 ├── Background Removal: segmentación semántica
 └── Text Rewrite: LLM o modelo local
```


## Modelo del documento

El documento no debe guardarse como imagen final ni como SVG plano. Debe ser una estructura serializable de objetos editables: páginas, capas, transformaciones, estilos y referencias a recursos. Canva trata el diseño como un árbol de elementos con orden Z, selección y operaciones de agrupación, movimiento y eliminación.[^2][^3]

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DesignDocument {
    pub id: Uuid,
    pub pages: Vec<Page>,
    pub assets: AssetStore,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Page {
    pub id: Uuid,
    pub width: f32,
    pub height: f32,
    pub background: Paint,
    pub layers: Vec<LayerNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LayerNode {
    Text(TextNode),
    Image(ImageNode),
    Shape(ShapeNode),
    Frame(FrameNode),
    Group(GroupNode),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ElementBase {
    pub id: Uuid,
    pub z: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation_deg: f32,
    pub opacity: f32,
    pub visible: bool,
    pub locked: bool,
}
```


### Texto

```rust
pub struct TextNode {
    pub base: ElementBase,
    pub content: String,
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: u16,
    pub color: Color,
    pub align: TextAlign,
    pub line_height: f32,
    pub letter_spacing: f32,
    pub effects: TextEffects,
}
```


### Imagen y marco

```rust
pub struct ImageNode {
    pub base: ElementBase,
    pub asset_id: Uuid,
    pub crop: Rect,
    pub filters: ImageFilters,
    pub fit: ImageFit,
}

pub struct FrameNode {
    pub base: ElementBase,
    pub frame_shape: FrameShape,
    pub child_asset: Option<Uuid>,
    pub child_crop: Rect,
    pub child_offset: Point,
    pub child_scale: f32,
}
```

Los marcos funcionan como contenedores: la imagen se recorta automáticamente a la forma del marco y el usuario puede ajustar posición, escala y porción visible sin modificar el marco.[^4][^5]

## Librerías Rust recomendadas

| Necesidad | Crate recomendado | Alternativas | Uso |
| :-- | :-- | :-- | :-- |
| GUI de escritorio | `egui` + `egui-wgpu` | `iced`, `slint`, `gpui` | Paneles, toolbars, diálogos, formularios y estado de UI. [^6] |
| Render 2D GPU | `wgpu` | `vello`, `skia-safe` | Lienzo, transformaciones, efectos, capas y render acelerado. [^7][^6] |
| Render vectorial | `vello` | `skia-safe`, `tiny-skia` | Formas, curvas Bézier, rellenos, trazos, sombras y máscaras. |
| SVG | `usvg` + `resvg` | `svg` | Importar, normalizar y rasterizar SVG. |
| Imágenes | `image` | `imageproc` | Decodificación, recorte, resize, RGBA, filtros y exportación. |
| Fondo removido | `rembg-rs` o `rmbg` | `ort` + modelo U2-Net/ISNet propio | Segmentación con ONNX Runtime y generación de máscara alfa. [^8][^9][^10] |
| Inferencia ONNX | `ort` | `candle`, `tract` | Ejecutar modelos de segmentación o OCR. [^8][^9] |
| Texto avanzado | `cosmic-text` o `parley` | `ab_glyph`, `fontdue` | Shaping, fuentes variables, RTL, salto de línea y medición. |
| PDF | `printpdf` | `lopdf` | Generación de PDF para impresión. |
| Base de datos local | `sqlite` via `rusqlite` | `redb`, `sled` | Proyectos, assets, historial y metadatos. |
| Estado e historial | `im` / `im-rc` | `rpds` | Estructuras persistentes para undo/redo barato. |
| Serialización | `serde` + `serde_json` | `postcard` | Guardar documentos, plantillas y comandos. |
| Tareas asíncronas | `tokio` | `async-std` | Subidas, exportación, IA y sincronización. |

Para un editor serio, la combinación más equilibrada es **`egui` para UI + `wgpu`/`vello` para lienzo + `cosmic-text` para texto + `image` para raster + `ort` para IA**. `egui` es una GUI inmediata, portable y compatible con `wgpu`, mientras que `wgpu` permite construir un renderizador propio para documentos con muchas capas.[^6][^7]

## Motor de renderizado

### Estrategia de render

| Enfoque | Cuándo usarlo | Ventaja | Riesgo |
| :-- | :-- | :-- | :-- |
| **SVG / DOM** | Pocos elementos y UI accesible | Escalable, accesible, fácil hit-testing | Costoso con miles de nodos. [^3] |
| **Canvas 2D** | Cientos de formas simples | Simple y rápido en CPU | Redibujado costoso y menos adecuado para efectos avanzados. [^3] |
| **WebGPU / wgpu** | Miles de elementos, efectos, sombras y video | Alta performance y control total | Complejidad alta; hit-testing debe implementarse. [^3] |

Para replicar Canva en Rust, lo ideal es un render híbrido:

- **Capa de documento:** `wgpu` + `vello` o `skia-safe`.
- **Capa de selección y guías:** overlay independiente.
- **Capa de UI:** `egui`.
- **Capa de previsualización:** framebuffer o textura temporal para strokes, arrastres y transformaciones en vivo.

Canva usa una separación similar entre la superficie visual, los eventos y la lógica de herramienta; la previsualización se muestra antes de confirmar el cambio en el documento.[^1]

### Pipeline de frame

```text
1. Input: mouse, teclado, touch
2. Intent: seleccionar / mover / escalar / escribir
3. Command: aplicar operación al documento
4. Dirty regions: calcular qué capas cambiaron
5. Layout: texto, wrapping, alineación
6. Raster/GPU render: fondo, capas, efectos, overlay
7. Present: presentar frame
8. Persist: guardar comando y versión
```


## Algoritmos críticos

### 1. Hit-testing

Determina qué elemento fue seleccionado al hacer clic. Para rectángulos rotados, se transforma el punto del mouse al espacio local del elemento.

Dado un punto global $p$, un elemento con posición $t$, tamaño $w \times h$ y rotación $\theta$:

$$
p_{local} = R(-\theta) \cdot (p - t)
$$

El punto está dentro si:

$$
0 \le p_{local,x} \le w
\quad \land \quad
0 \le p_{local,y} \le h
$$

En Rust, el orden de evaluación debe ser de la capa superior a la inferior, respetando el valor `z`.

```rust
pub fn hit_test(point: Point, element: &ElementBase) -> bool {
    let local = inverse_rotate(point - element.position(), element.rotation_deg);
    local.x >= 0.0 && local.x <= element.width
        && local.y >= 0.0 && local.y <= element.height
}
```


### 2. Transformación affine

Toda manipulación visual —mover, escalar, rotar— puede representarse con una matriz afín:

$$
M =
\begin{bmatrix}
a & c & e \\
b & d & f \\
0 & 0 & 1
\end{bmatrix}
$$

Para rotación alrededor del centro:

$$
x' = cx + (x - cx)\cos\theta - (y - cy)\sin\theta
$$

$$
y' = cy + (x - cx)\sin\theta + (y - cy)\cos\theta
$$

### 3. Escalado desde handle

Al arrastrar una esquina, la escala se calcula respecto al punto opuesto, no respecto al centro:

$$
scale_x = \frac{|handle_x - anchor_x|}{|start_x - anchor_x|}
$$

$$
scale_y = \frac{|handle_y - anchor_y|}{|start_y - anchor_y|}
$$

Si se mantiene proporción:

$$
scale = \max(scale_x, scale_y)
$$

### 4. Snap y guías

El *snapping* alinea un elemento con bordes, centros u otros elementos cuando la distancia es menor que un umbral.

$$
if\ |edge_a - edge_b| < \epsilon \Rightarrow edge_a = edge_b
$$

Un umbral típico en pantalla es $4$ a $8$ px. Deben calcularse guías para:

- Borde izquierdo, centro y borde derecho.
- Borde superior, centro y borde inferior.
- Centros de página y de otros elementos.
- Espaciados iguales entre elementos.


### 5. Undo / redo

La mejor arquitectura es **Command Sourcing**: cada edición genera un comando inverso.

```rust
pub enum Command {
    AddElement { page: Uuid, node: LayerNode },
    RemoveElement { page: Uuid, node: LayerNode },
    MoveElement { id: Uuid, from: Point, to: Point },
    ResizeElement { id: Uuid, from: Rect, to: Rect },
    RotateElement { id: Uuid, from: f32, to: f32 },
    ChangeColor { id: Uuid, from: Color, to: Color },
    SetText { id: Uuid, from: String, to: String },
}
```

Cada comando implementa:

```rust
pub trait Command {
    fn apply(&mut self, doc: &mut DesignDocument);
    fn revert(&mut self, doc: &mut DesignDocument);
}
```

Para evitar memoria excesiva, agrupar comandos continuos: por ejemplo, un arrastre completo se confirma como un solo `MoveElement`, no uno por frame.

### 6. Layout de texto

El ajuste de línea depende del ancho disponible y del ancho medido de cada palabra:

```text
line_width = 0
for word in words:
    candidate = line_width + space_width + word_width
    if candidate <= max_width:
        add word to current line
    else:
        close current line
        start new line with word
```

La altura final es:

$$
H = lines \times font\_size \times line\_height
$$

Para texto con kerning y shaping real, usar `cosmic-text` o `parley`; medir con `ab_glyph` solamente es insuficiente para tipografía profesional multilingüe.

### 7. Recorte dentro de un frame

El frame define un rectángulo o forma destino. La imagen se transforma para cubrirlo manteniendo proporción:

$$
scale = \max\left(\frac{frame_w}{image_w}, \frac{frame_h}{image_h}\right)
$$

Luego se aplica un recorte:

$$
crop_w = \frac{frame_w}{scale}
$$

$$
crop_h = \frac{frame_h}{scale}
$$

Esto equivale a un *cover fit* con offset controlable por el usuario.

### 8. Exportación

La exportación debe renderizar el documento a un *offscreen target* con el tamaño final solicitado.

```text
document -> scene graph -> render target -> encoder -> archivo
```

| Salida | Proceso |
| :-- | :-- |
| PNG | Render RGBA a 1x, 2x o 3x; codificar con `image`. [^11][^12] |
| JPG | Render RGB, aplicar calidad, descartar alfa. [^11][^12] |
| SVG | Serializar formas, textos e imágenes a SVG; textos pueden rasterizarse si hay fuentes complejas. [^11][^12] |
| PDF | Render vectorial o raster embebido; usar `printpdf` o `lopdf`. [^11][^13] |
| MP4 | Render de frames y codificación con `ffmpeg-sidecar` o bindings de FFmpeg. [^11][^13] |
| GIF | Generar frames y cuantizar paleta. [^11] |

## Eliminación de fondo en Rust

Canva usa IA para identificar el sujeto principal y separar el fondo automáticamente. Técnicamente, el flujo estándar es segmentación semántica: normalizar la imagen, ejecutar un modelo y convertir la máscara resultante en un canal alfa.[^9][^14]

### Pipeline

```text
Imagen RGB
 → resize al tamaño del modelo
 → normalización
 → modelo de segmentación U2-Net / ISNet
 → máscara 1 canal
 → suavizado / threshold / feather
 → máscara alfa RGBA
 → PNG transparente
```


### Fórmulas

Normalización habitual:

$$
x_{norm} = \frac{x - \mu}{\sigma}
$$

Conversión de máscara a alfa:

$$
\alpha = clamp(mask, 0, 1) \times 255
$$

Composición final:

$$
C_{out} = C_{fg} \cdot \alpha + C_{bg} \cdot (1 - \alpha)
$$

Para exportar con fondo transparente, se conserva el canal alfa; para exportar JPG, el alfa se compone contra blanco u otro color de fondo.

### Crates

| Crate | Rol |
| :-- | :-- |
| `rembg-rs` | Implementación Rust de eliminación de fondo con ONNX Runtime y U2-Net. [^8] |
| `rmbg` | Alternativa con carga y aplicación de modelo de segmentación. [^10] |
| `ort` | Runtime ONNX para ejecutar modelos. [^8][^9] |
| `image` | Decodificación, conversión RGBA y guardado PNG. |
| `ndarray` | Preprocesamiento de tensores. |
| `tokenizers` | Solo si se integra un modelo de lenguaje local para reescritura. |

### Tiempo esperado

| Imagen | Tiempo típico |
| :-- | --: |
| Foto estándar, CPU | 2–8 segundos |
| Foto estándar, GPU | 0.3–2 segundos |
| Imagen compleja o alta resolución | 5–15 segundos |

Canva reporta tiempos aproximados de 2 a 5 segundos para tareas estándar, aunque el resultado depende de resolución, sujeto y complejidad del fondo.[^15]

## Reescritura de texto

Para el alcance definido, no se necesita generación de diseño completo: solo mejorar o reformular textos cortos.

### Opciones

| Opción | Ventaja | Desventaja |
| :-- | :-- | :-- |
| API de LLM | Mejor calidad y contexto | Costo, latencia y dependencia externa |
| Modelo local pequeño | Privacidad y costo bajo | Requiere hardware y calidad variable |
| Reglas + diccionario | Instantáneo y barato | Limitado a sinónimos y limpieza básica |

### Pipeline local

```text
Texto original
 → detección de idioma
 → prompt con instrucción y límite de caracteres
 → modelo local o API
 → validación de longitud
 → reemplazo en el TextNode
 → comando undo/redo
```


### Tiempo esperado

| Texto | API LLM | Modelo local |
| :-- | --: | --: |
| Título corto | 0.5–2 s | 0.3–3 s |
| Caption de 1–2 frases | 1–4 s | 1–8 s |
| Párrafo corto | 2–8 s | 3–20 s |

## UI/UX de Canva: composición y estructura

Los análisis de UX coinciden en que Canva se basa en **drag-and-drop, WYSIWYG, plantillas y una interfaz de baja carga cognitiva**. Su patrón principal es: panel lateral para recursos, lienzo central para manipulación directa y barra contextual para propiedades del elemento seleccionado.[^16][^17]

### Estructura de pantalla

```text
┌────────────────────────────────────────────────────────────┐
│ Top bar: nombre, undo/redo, zoom, compartir, exportar      │
├──────────┬──────────────────────────────────┬──────────────┤
│ Sidebar  │           Canvas                 │ Context bar  │
│          │                                  │              │
│ Diseño   │   ┌────────────────────────┐     │ Texto        │
│ Elementos│   │      Página activa      │     │ Color        │
│ Texto    │   │  [texto] [imagen] [forma]│    │ Filtros      │
│ Subidas  │   └────────────────────────┘     │ Capas        │
│ Marcos   │                                  │ Ajustes      │
└──────────┴──────────────────────────────────┴──────────────┘
```


### Principios de UX observados

| Principio | Cómo se materializa en Canva | Cómo replicarlo |
| :-- | :-- | :-- |
| **Cero página en blanco** | Plantillas sugeridas desde el inicio | Abrir siempre con plantillas por intención, tamaño y categoría. [^16][^18] |
| **Manipulación directa** | Arrastrar, escalar y rotar sobre el lienzo | Selección con handles, snapping y preview en vivo. [^1][^17] |
| **Contextualidad** | La barra cambia según texto, imagen o forma | Un solo `ContextToolbar` alimentado por el tipo de selección. [^19][^20] |
| **Progresión simple** | Opciones básicas primero; funciones avanzadas ocultas | Menús progresivos y paneles colapsables. [^17][^19] |
| **Feedback inmediato** | Cambios visibles en tiempo real | Render incremental y preview antes de commit. [^1][^18] |
| **Consistencia visual** | Paleta sobria, gradientes de marca y jerarquía clara | Design tokens: color, spacing, radius, tipografía y elevación. [^21][^22] |

### Composición visual

Canva utiliza una base predominantemente blanca y gris, con tipografía sans-serif oscura y acentos de color limitados; su gradiente característico aparece como identidad de marca más que como color funcional en toda la interfaz.[^21]

Recomendación de tokens:

```json
{
  "color.surface": "#FFFFFF",
  "color.surface-muted": "#F5F6F8",
  "color.text-primary": "#0D1216",
  "color.text-secondary": "#5F6B76",
  "color.accent": "#8B3DFF",
  "color.selection": "#8B3DFF",
  "radius.control": "8px",
  "radius.panel": "12px",
  "spacing.xs": "4px",
  "spacing.sm": "8px",
  "spacing.md": "16px",
  "spacing.lg": "24px"
}
```


### Interacciones críticas

| Interacción | Comportamiento esperado |
| :-- | :-- |
| Clic | Selecciona el elemento superior bajo el cursor. [^23][^2] |
| Doble clic en texto | Entra en modo edición de texto. [^24] |
| Arrastre | Mueve el elemento con preview y snapping. [^1][^17] |
| Handles de esquina | Escala; con `Shift`, mantiene proporción. |
| Handles laterales | Escala un eje. |
| Handle de rotación | Rota alrededor del centro. |
| `Ctrl/Cmd + Z` | Deshace el último comando. |
| `Ctrl/Cmd + Shift + Z` | Rehace. |
| `Ctrl/Cmd + D` | Duplica selección. |
| `Delete` | Elimina elemento, con posibilidad de deshacer. |
| Arrastre de imagen a frame | La imagen se recorta automáticamente a la forma. [^4][^5] |

## Hallazgos de estudios de usabilidad

Los estudios disponibles muestran una paradoja: Canva se percibe como fácil, rápida y satisfactoria, pero tiene problemas de **memorabilidad de íconos**, recuperación de errores y descubrimiento de funciones.[^25][^26][^27]


| Hallazgo | Evidencia | Implicación de diseño |
| :-- | :-- | :-- |
| Alta facilidad de uso | 77.5% de estudiantes percibió facilidad de uso. [^25] | Priorizar plantillas, drag-and-drop y acciones visibles. |
| Rapidez percibida | 65% percibió rapidez en las funciones. [^25] | Mantener respuestas inferiores a 100 ms para interacciones locales. |
| Buena memorabilidad general | 72.5% recordaba bien los pasos. [^25] | Usar flujos lineales y consistentes. |
| Íconos poco claros | 47.5% tuvo dificultad reconociendo íconos. [^25] | Combinar icono + etiqueta en funciones importantes. |
| Errores difíciles de corregir | 62.5% encontró errores; solo 50% los corrigió fácilmente. [^25] | Undo global, mensajes accionables y restauración de estado. |
| Alta satisfacción | 92.5% se declaró satisfecho. [^25] | El valor percibido depende más de resultado rápido que de control avanzado. |
| Plantillas percibidas como útiles | La investigación sobre plantillas muestra que mejoran usabilidad y reducen dificultad. [^28] | Tratar plantillas como producto, con metadatos, pruebas y analítica. |
| Paywall interrumpe el flujo | Un audit señala que elementos Pro pueden bloquear el editor con un modal. [^26] | Mostrar preview, permitir reemplazo por alternativa gratuita y no destruir contexto. |
| Navegación de plantillas puede abrumar | La galería plana dificulta encontrar categorías. [^26] | Búsqueda por intención, filtros y plantillas contextuales. |

## Diagrama de flujo de edición

```text
Inicio
  ↓
Seleccionar intención
  ↓
Buscar plantilla por necesidad
  ↓
Abrir documento
  ↓
Seleccionar elemento
  ↓
¿Qué tipo es?
  ├── Texto → editar contenido, fuente, color, efectos
  ├── Imagen → subir, recortar, filtrar, eliminar fondo
  ├── Forma → cambiar color, borde, tamaño
  └── Frame → arrastrar imagen, ajustar crop
  ↓
Preview en vivo
  ↓
Commit del comando
  ↓
Undo disponible
  ↓
Exportar PNG / JPG / PDF / SVG / MP4
```


## Requisitos mínimos para un clon viable

### MVP funcional

1. Documento con páginas, capas, texto, imágenes, formas y marcos.
2. Selección, hit-testing, mover, escalar, rotar, duplicar y eliminar.
3. Undo/redo basado en comandos.
4. Panel de plantillas y biblioteca de assets.
5. Editor contextual para texto, color, imagen y marco.
6. Snap básico, guías y alineación.
7. Eliminación de fondo con modelo ONNX.
8. Reescritura de texto corto.
9. Exportación PNG, JPG y PDF.
10. Autoguardado local con SQLite o archivo `.design`.

### Métricas de performance objetivo

| Acción | Objetivo |
| :-- | --: |
| Selección | < 16 ms |
| Arrastre de elemento | 60 fps |
| Cambio de color | < 50 ms percibido |
| Aplicar filtro | < 300 ms para imagen mediana |
| Eliminar fondo | < 5 s en CPU; < 2 s con GPU |
| Reescribir texto corto | < 4 s |
| Exportar PNG 1080p | < 2 s |
| Abrir documento de 100 capas | < 500 ms |

## Conclusión técnica

Para replicar el funcionamiento esencial de Canva, no conviene copiar su superficie completa, sino implementar primero su **bucle de valor**: plantilla → selección → edición directa → preview → commit → exportación. En Rust, la base más sólida es `egui` para la interfaz, `wgpu`/`vello` para el lienzo, `cosmic-text` para tipografía, `image` para raster, `usvg`/`resvg` para SVG y `ort` con U2-Net/ISNet para eliminación de fondo.[^8][^6][^9][^1]

La diferencia entre un editor básico y uno percibido como “fácil como Canva” estará menos en la cantidad de funciones y más en: **plantillas relevantes, manipulación directa fluida, toolbar contextual, undo confiable, feedback inmediato y exportación sin fricción**.[^17][^16][^25]

<span style="display:none">[^29][^30][^31][^32][^33][^34][^35][^36][^37][^38][^39]</span>

<div align="center">⁂</div>

[^1]: https://www.canva.dev/blog/engineering/behind-the-draw/

[^2]: https://www.greatfrontend.com/interviews/company/canva/questions-guides

[^3]: https://www.techinterview.org/companies/canva/

[^4]: https://www.canva.com/en_gb/help/using-frames-variantb/

[^5]: https://www.canva.com/help/using-frames/

[^6]: https://crates.io/crates/egui/0.33.3

[^7]: https://crates.io/crates/oxui-wgpu/0.5.0

[^8]: https://docs.rs/crate/rembg-rs/latest

[^9]: https://github.com/i-rocky/rembg

[^10]: https://lib.rs/crates/rmbg

[^11]: https://www.canva.dev/docs/apps/exporting-designs/

[^12]: https://www.techspot.com/downloads/7510-canva.html

[^13]: https://www.canva.dev/docs/apps/rest-apis/reference/exports/

[^14]: https://www.canva.com/en_gb/help/background-remover-variantb/

[^15]: https://www.staymodern.ai/solutions/canva-background-remover/detailed

[^16]: https://www.datanext.ai/case-study/canva-online-design-tool/

[^17]: https://www.mindinventory.com/blog/saas-ui-ux-design-case-studies/

[^18]: https://api-elearning.pnb.ac.id/upload/answer/ANSW-1762877188-c74e13165d-2415754079.pdf

[^19]: https://screensdesign.com/showcase/canva-ai-photo-video-editor

[^20]: https://education.nsw.gov.au/teaching-and-learning/technology-for-learning/resources/professional-learning-resources/canva-for-education/canva-foundations/transcript-canva-editing-tools

[^21]: https://www.anthonythompson.design/canva

[^22]: https://elihochberg.net/Canva-UI-Editorial-System

[^23]: https://www.canva.com/help/using-layouts/

[^24]: https://www.canva.com/help/add-and-edit-text/

[^25]: https://www.unisbank.ac.id/ojs/index.php/fti1/article/view/9702

[^26]: https://releaselens.org/demos/canva/

[^27]: https://www.kimola.com/reports/consumer-insights-on-canvas-usability-and-features

[^28]: https://design.sallywoellner.com/ux-of-content/

[^29]: https://www.canva.dev/blog/engineering/

[^30]: https://www.canva.dev/blog/engineering/feed.xml

[^31]: https://geekyants.com/en-us/blog/building-a-production-ready-canva-like-editor-with-konva-js-react-19-and-next-js-15

[^32]: https://docs.rs/wgpu_canvas/latest/wgpu_canvas/

[^33]: https://crates.io/crates/egui/0.30.0

[^34]: https://d-research.or.kr/bbs/download2.php?bo_table=paper\&wr_id=2806\&no=3

[^35]: https://ojs.uniska-bjm.ac.id/index.php/PIUOK/article/viewFile/15601/6574

[^36]: https://jessicadinneen.com/CANVA-USABILITY-STUDY

[^37]: http://indojurnal.com/index.php/jejakdigital/article/download/1725/1494/4772

[^38]: https://vibecodeit.tech/canva

[^39]: https://platformchecker.com/blog/canva-tech-stack-2026

