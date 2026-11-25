# 100 — Objetos 3D: importación y edición básica

> **Estado:** in-progress — 2026-11-25 — rama `feat/100-objetos-3d`; fase 1 con render 3D por CPU (D-130).

**Versión:** 1.0  
**Plataforma:** aplicación de escritorio nativa en Rust (Windows, Linux y macOS)  
**Alcance:** importación `.glb` y `.obj`, edición 3D encapsulada, integración con lienzo 2D y exportación visual  
**Prioridad:** Funcionalidad básica primero; extensibilidad sin complejidad visible para el usuario

## 1. Visión

Gafira debe admitir modelos 3D como **elementos gráficos del lienzo 2D**. El usuario importa un archivo `.glb` o `.obj`, lo inserta en el diseño, lo redimensiona y mueve como cualquier imagen o vector. Al activar la edición 3D, accede a un espacio encapsulado para orientar, girar, desplazar y escalar el modelo de manera intuitiva. Tras aplicar los cambios, el lienzo representa **únicamente la proyección visible del modelo desde la cámara**, con transparencia alrededor, sin mostrar el escenario, guías o controles 3D.

**Principio de producto:** Gafira no se convierte en un modelador 3D. La experiencia es similar a manipular un elemento gráfico con perspectiva editable.

**Distinción importante:** la representación dentro del lienzo puede ser un bitmap RGBA con canal alfa generado a partir del objeto 3D; **no es geometría vectorial SVG**. Debe comportarse como un elemento gráfico estándar para transformar, alinear, recortar, superponer y exportar. La editabilidad 3D debe conservarse en el proyecto.

## 2. Objetivos y límites

### 2.1. Objetivos

- Importar modelos locales `.glb` y `.obj` mediante diálogo de archivos o arrastrar y soltar.
- Visualizar el modelo completo en el lienzo, con tamaño inicial apropiado, correctamente iluminado y sin fondo.
- Incorporar controles habituales del editor 2D: selección, mover, escalar, duplicar, alinear, bloquear, orden de capas, opacidad y eliminar.
- Acceder al **modo de edición 3D encapsulado** mediante doble clic o botón contextual «Editar 3D»; un clic simple mantiene la selección 2D habitual.
- Manipular la orientación del objeto con arrastre, controles visuales, vistas predefinidas y botón de restablecimiento.
- Permitir modificar posición y escala dentro del encuadre 3D sin cambiar la posición del elemento en el lienzo.
- Conservar geometría, cámara, transformaciones, iluminación y apariencia para reabrir y editar sin pérdidas.
- Exportar PNG, JPEG, WebP y PDF con composición visual consistente: solamente los píxeles visibles, con o sin transparencia según el formato.
- Funcionar sin servicios externos ni conexión a Internet después de instalar Gafira.

### 2.2. Fuera de alcance de la primera versión

- Edición de vértices, mallas, topología, UV, texturas o materiales avanzados.
- Modelado 3D, esculpido, animación, física o simulación.
- Exportación de los modelos 3D modificados como OBJ/GLB desde Gafira.
- Edición individual de huesos o submallas; la transformación se aplica al modelo como unidad.
- Iluminación artística con múltiples fuentes configurables.
- Exportación 3D vectorial real; la salida SVG/PDF incorpora imagen rasterizada, no mallas convertidas a vectores.

## 3. Experiencia de usuario

### 3.1. Importación

1. El usuario abre **Elementos > 3D > Importar modelo** o arrastra un `.glb`/`.obj` sobre el lienzo.
2. Gafira valida formato, tamaño, contenido y dependencias antes de añadirlo.
3. Muestra progreso y permite cancelar la carga; si falla, comunica el motivo sin interrumpir el diseño.
4. Al terminar, normaliza visualmente el encuadre: centra el modelo según sus límites geométricos, calcula un ajuste de cámara y aplica iluminación de estudio neutra.
5. Inserta un nuevo elemento seleccionado en el centro visible del lienzo, respetando límites del documento y orden de capas.
6. Muestra una vista inicial transparente del modelo y el menú contextual «Editar 3D».

**Archivos OBJ asociados:** si el OBJ utiliza `.mtl` y texturas externas, permitir seleccionarlos conjuntamente o importar una carpeta/ZIP controlado; resolver referencias **solo dentro del conjunto elegido por el usuario**, sin rutas externas arbitrarias. Si faltan materiales, importar geometría con material neutro y mostrar advertencia no bloqueante. El soporte `.mtl` puede ser una mejora de fase 2, pero la ausencia de texturas no debe impedir importar la geometría.

**GLB:** soportar glTF 2.0 GLB estándar. Evaluar extensiones y compresiones (por ejemplo Draco, Meshopt y KTX2) como capacidades opcionales explícitas; no afirmar soporte si no están instalados los decodificadores.

### 3.2. Comportamiento en el lienzo 2D

- **Un clic:** selecciona el elemento como objeto 2D y muestra su rectángulo delimitador.
- **Arrastrar desde su cuerpo:** lo mueve en el plano del lienzo.
- **Tiradores 2D:** cambian su tamaño; respetar relación de aspecto por defecto.
- **Doble clic o «Editar 3D»:** abre un entorno 3D acotado al elemento.
- **Salir de edición:** vuelve al lienzo normal, conservando la vista seleccionada.
- Al duplicar, ambos elementos pueden compartir el mismo recurso binario base, pero **cada instancia conserva su propia cámara, transformación, aspecto y vista previa**.
- Rotación 2D del elemento y rotación 3D del modelo son operaciones distintas: la primera gira la proyección completa dentro del lienzo; la segunda cambia la cara del objeto que mira hacia el observador.
- El elemento interactúa con capas, alineación, distribución, recorte, máscaras, sombras 2D, opacidad, agrupación, deshacer/rehacer e historial igual que el resto de elementos, en la medida en que el motor de composición existente lo admita.

### 3.3. Editor 3D encapsulado

El modo de edición debe mostrarse preferentemente como un **panel contextual superpuesto, acoplado al elemento o ampliable**, sin abandonar el documento. No se debe reemplazar toda la interfaz de Gafira por un editor tridimensional generalista.

Componentes del panel:

- Área de vista 3D con fondo de previsualización neutro o cuadriculado opcional (no exportable).
- Barra compacta: **Girar · Mover · Escalar · Encuadrar · Restablecer**.
- Control de orientación (gizmo/cubo de ejes) sencillo para elegir frente, atrás, izquierda, derecha, arriba e isométrica.
- Controles «Acercar/Alejar», «Ajustar modelo», «Centrar» y «Revertir».
- Conmutador **Perspectiva / Ortográfica**; comenzar con perspectiva como valor predeterminado y ortográfica como alternativa.
- Botones **Aplicar** y **Cancelar** claramente visibles; la cancelación restaura el estado previo de esta sesión de edición.
- Opciones avanzadas plegadas por defecto para X/Y/Z, ángulos, escala y cámara.

**Entrada del modo:** mostrar sugerencia discreta «Arrastra para girar». Mantener atajos visibles o ayuda contextual. Evitar ejes, cuadrículas o paneles profesionales por defecto.

### 3.4. Semántica de controles (no ambigua)

| Acción | Interacción principal | Efecto |
|---|---|---|
| Girar modelo | Arrastre izquierdo en modo Girar | Rota el modelo sobre ejes locales o de vista con comportamiento tipo trackball limitado, sin cambiar el elemento 2D |
| Giro preciso | Controles X, Y, Z o gestos con modificadores | Orientación reproducible; ángulos normalizados |
| Mover modelo | Arrastre en modo Mover | Traslada la geometría dentro del encuadre de su propia cámara |
| Escalar modelo | Deslizador o arrastre del control de escala | Cambia el tamaño aparente en el encuadre 3D, sin alterar las dimensiones 2D |
| Zoom / cámara | Rueda, gesto táctil o botones +/− | Ajusta distancia o zoom de cámara; **no** escala el elemento del lienzo |
| Encuadrar | Botón «Ajustar» | Reencuadra el modelo para dejarlo completamente visible |
| Restablecer | Botón contextual | Restituye cámara y transformación al estado inicial de importación |
| Vistas rápidas | Botones Frente, Lado, Arriba, Iso | Cambian la orientación visible de forma determinista |

**Regla de usabilidad:** no reutilizar el mismo arrastre para mover el elemento 2D y girar el modelo 3D. Cada gesto opera únicamente en su modo activo. En modo 3D, la superficie de interacción debe capturar eventos y evitar la selección o movimiento del lienzo que esté detrás.

**Controles móviles:** soporte táctil con un dedo para giro y gesto de pellizco para zoom; ofrecer botón visible de modo Mover para no competir con los gestos. Mantener controles accesibles desde teclado.

### 3.5. Estado de edición

- **Aplicar:** confirma rotación, traslación, escala y cámara; actualiza la proyección RGBA y registra una entrada de historial atómica.
- **Cancelar/Escape:** descarta cambios de la sesión; no toca el resto de elementos del lienzo.
- **Deshacer/Rehacer:** funciona tanto después de aplicar como para acciones dentro del editor cuando este está abierto.
- **Cambio de selección:** solicitar aplicar o descartar únicamente si existen modificaciones no confirmadas, evitando cuadros de diálogo innecesarios.
- **Errores del renderer:** mantener la última previsualización válida y ofrecer reintentar sin perder el proyecto.

## 4. Modelo de datos del elemento

Usar un tipo nuevo `3d-model` o el discriminante equivalente del motor de documentos de Gafira; integrar sin romper los tipos existentes.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model3DElement {
    pub id: String,
    pub name: String,
    pub transform_2d: ElementTransform2D,
    pub model: Model3DInstance,
    pub view: Model3DView,
    pub preview: Model3DPreview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementTransform2D {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation_degrees: f32,
    pub opacity: f32,
    pub visible: bool,
    pub locked: bool,
    pub z_index: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Model3DFormat { Glb, Obj }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model3DInstance {
    pub asset_id: String,
    pub source_format: Model3DFormat,
    pub source_name: String,
    pub content_hash: String,
    pub position: [f32; 3],
    pub rotation_quaternion_xyzw: [f32; 4],
    pub scale: [f32; 3],
    pub origin_offset: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CameraKind { Perspective, Orthographic }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model3DView {
    pub camera_kind: CameraKind,
    pub camera_position: [f32; 3],
    pub camera_target: [f32; 3],
    pub camera_up: [f32; 3],
    pub fov_degrees: f32,
    pub ortho_vertical_span: Option<f32>,
    pub auto_fit: bool,
    pub transparent_background: bool,
    pub lighting_preset: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model3DPreview {
    pub asset_id: Option<String>,
    pub revision: u64,
}
```

**Normas:**

- No serializar recursos vivos `wgpu::Device`, `wgpu::Texture`, buffers GPU, handles de ventana ni objetos de escena en el `.gaf`; persistir únicamente datos serializables con `serde`.
- La geometría original se mantiene inmutable. Todas las transformaciones y opciones de vista pertenecen a la instancia.
- Separar `x/y/width/height/rotation` 2D de `model.position/quaternion/scale` 3D.
- Guardar la representación rotacional como quaternion para evitar ambigüedad de orden Euler; exponer ángulos X/Y/Z solo en la interfaz, mediante conversión controlada.
- `preview.assetId` es una caché regenerable, no la fuente de verdad.
- El esquema debe ser versionado y admitir migraciones.

## 5. Persistencia en archivos `.gaf`

El modelo deberá conservarse **dentro del proyecto portable `.gaf`**, siguiendo el mecanismo de recursos del contenedor existente; los paths externos se conservan solo como metadatos de procedencia, nunca como requisito para reabrir el diseño.

Estructura conceptual (adaptar a la estructura real, sin duplicar manifiestos):

```text
mi-diseno.gaf
├── manifest.json
├── document.json
├── assets/
│   └── 3d/
│       ├── <sha256>.glb
│       └── <sha256>.obj
├── dependencies/
│   └── 3d/<asset-id>/
│       ├── material.mtl
│       └── textures/...
└── cache/
    └── 3d-previews/<instance-id>.png
```

- Los archivos `.obj` con dependencias se empaquetan junto a sus materiales y texturas con rutas **relativas saneadas**.
- Se reutiliza el mismo recurso binario para instancias duplicadas mediante hash; no duplicar geometría en cada elemento.
- Los recursos originales pueden comprimirse dentro del contenedor, sin conversiones destructivas automáticas.
- El guardado es atómico y consistente; los thumbnails/cachés pueden reconstruirse cuando cambie la versión del renderer.
- Si la versión de Gafira no soporta 3D o falla el motor de renderizado, mostrar la previsualización persistida y preservar sin cambios los datos originales al guardar, siempre que el formato del proyecto permita conservación de campos desconocidos.

## 6. Arquitectura técnica recomendada

### 6.1. Stack nativo Rust

> **D-130:** el titular eligió un rasterizador 3D propio por CPU en lugar de `wgpu`, porque el lienzo de Gafira usa glow y su render es por CPU. Donde esta sección y la 6.3 hablan de `wgpu`, de texturas GPU o de pérdida de dispositivo, se aplica el rasterizador por CPU: mismo encuadre y color en vista previa y exportación, sin dispositivo GPU que perder.

**Principio obligatorio:** Gafira es una aplicación nativa en Rust. No introducir Three.js, WebGL, JavaScript, Electron, servidor web ni WebView para implementar el módulo 3D.

| Responsabilidad | Recomendación Rust | Criterio |
|---|---|---|
| GPU/renderizado 3D | `wgpu` | Compatible con backends nativos, integración con texturas GPU y render targets offscreen |
| Importar GLB | `gltf` | Lectura de escenas, mallas, materiales y referencias glTF 2.0; decodificar imágenes según funcionalidades usadas |
| Importar OBJ | `tobj` | Lectura de mallas OBJ y soporte de materiales MTL sujeto a validación |
| Álgebra 3D | `glam` | Vectores, matrices y quaternions para cámara y transformaciones |
| Serialización | `serde` | Estado del elemento 3D portable y versionado dentro de `.gaf` |
| Caché de previews | `image` o pipeline de imágenes existente | RGBA y PNG con alfa; sin necesidad de crear una ventana |
| Renderizador de nivel superior | Opcional, **evaluar** `rend3` o alternativas mantenidas | Solo si evita implementar desde cero PBR, materiales y pipeline glTF; no incorporarlo sin comprobar mantenimiento y compatibilidad |

**Decisión técnica:** aprovechar el `wgpu::Device` y la cola GPU ya existentes **si el motor de Gafira utiliza wgpu**. Si el lienzo emplea otro backend, crear un adaptador nativo que comparta la imagen RGBA o textura mediante el mecanismo soportado; no asumir interoperabilidad automática entre APIs GPU. No crear un dispositivo GPU por objeto.

**Materiales y compatibilidad:** definir un subconjunto probado de materiales glTF PBR (base color, metalness, roughness, normal maps y transparencia) y degradación visual explícita para extensiones no soportadas. OBJ sin MTL utiliza material neutro. Las extensiones GLB Draco/Meshopt/KTX2 requieren decodificadores compatibles y empaquetados, no se consideran implícitas.

### 6.2. Capas y responsabilidades

```text
Gafira Desktop (Rust)
├── Editor2D / Canvas / Layers
│   ├── Model3DElement (transformación 2D + referencia a preview)
│   ├── Model3DInspector (propiedades)
│   └── Model3DOverlay (editor 3D encapsulado)
│       └── InteractionController (drag, órbita, pan, zoom, gizmos)
├── AssetManager / GafContainer
│   ├── GlbImporter (crate gltf)
│   ├── ObjImporter (crate tobj)
│   └── Model3DAssetStore (originales, dependencias y hashes)
└── Model3DService
    ├── SceneGraph + Camera + Lighting (glam)
    ├── Renderer3D (wgpu; reutilizable)
    ├── OffscreenRenderTarget (RGBA + depth)
    ├── PreviewCache (versionado e invalidación)
    └── Model3DExportAdapter (compositor/exportación 2D)
```

Los controles visuales se implementan en el sistema UI **ya existente** de Gafira (por ejemplo `egui`, si es el utilizado), en vez de adoptar otro framework. La lógica de interacción y cámara no debe acoplarse al widget concreto, para permitir pruebas automatizadas y futuras adaptaciones.

### 6.3. Ciclo de renderizado

1. **Importar:** parsear en Rust, validar entradas, normalizar bounds/orientación y crear la escena mínima; generar preview RGBA.
2. **Lienzo normal:** mostrar la textura o bitmap RGBA cacheado en el compositor 2D existente, con las transformaciones del elemento; no renderizar 3D continuamente por cada objeto.
3. **Editar:** activar una única sesión de render 3D reutilizable y controles de cámara/modelo, dentro del overlay asociado al elemento.
4. **Aplicar:** serializar transformación 3D y cámara; invalidar caché; producir nueva vista y una sola operación de historial.
5. **Exportar:** renderizar offscreen a una resolución adecuada al tamaño real de salida, respetando escala/DPI, fondo transparente, canal alfa y color; componer con las capas, clipping y transformaciones 2D.
6. **Finalizar:** liberar buffers y texturas transitorias, sin destruir originales ni cachés persistentes.

**Rendimiento:** usar un solo servicio de render 3D y cachear previews por `asset hash + transformación + cámara + iluminación + resolución + versión de renderer`. Un documento con múltiples objetos 3D no debe crear un renderer ni una ventana por elemento.

**Offscreen:** renderizar a `wgpu::Texture` con color RGBA y depth buffer, leer la textura a CPU únicamente cuando sea necesario para un exportador o caché que no admita composición directa en GPU. Respetar alineación de filas para copy/readback, resolver MSAA cuando aplique y procesar alfa correctamente (premultiplicado vs straight alpha según contrato del compositor).

**Sistemas operativos:** probar backend GPU nativo en Windows, Linux y macOS, incluida creación y reconstrucción de recursos tras pérdida de dispositivo y fallback a la última preview válida. No requerir servicios externos o red para abrir y editar modelos locales.

## 7. Encapsulación y transparencia

- El viewport 3D está subordinado al elemento 2D, sin afectar el fondo del documento ni los demás elementos.
- El fondo del render final será **RGBA transparente**, aun si en el editor se presenta fondo gris o cuadrícula.
- Los gizmos, contornos, luces visuales, guías, rejilla, ejes y controles **nunca** forman parte del snapshot exportado.
- Respetar oclusión, materiales, canal alfa y profundidad dentro del objeto.
- Manejar materiales de doble cara y transparencias de forma compatible con el subconjunto de materiales glTF/OBJ implementado; documentar excepciones de materiales no soportados.
- La sombra de apoyo 3D no debe aparecer por defecto. Si se introduce en el futuro, será una opción explícita; las sombras 2D existentes pueden aplicarse a la silueta renderizada.
- **El rectángulo del elemento define el encuadre y los límites de interacción**, no necesariamente el área opaca de la figura. Si el modelo se sale del encuadre por configuración de cámara, recortar al rectángulo conforme a la semántica visual especificada.

## 8. Exportación 2D

### 8.1. Regla de oro

**Exportar exclusivamente lo que ve la cámara del modelo como imagen 2D dentro del lienzo.** No exportar mallas 3D ni vistas invisibles, caras ocultas o superficies fuera del encuadre.

- **PNG:** conserva fondo transparente y bordes suavizados.
- **WebP:** conserva transparencia cuando se utiliza variante con alfa.
- **JPEG:** composición sobre el fondo final del diseño, pues JPEG no admite alfa.
- **PDF:** inserta la proyección rasterizada de alta resolución dentro de la página; los demás elementos pueden seguir siendo vectoriales si el exportador lo permite.
- **SVG (si Gafira admite exportarlo):** incluir la proyección 3D como imagen incrustada o vinculada según la política de portabilidad del exportador, **sin afirmar que sea vectorial**.

### 8.2. Calidad y consistencia

- El tamaño de snapshot se calcula desde las dimensiones finales del elemento y el factor de exportación, respetando un límite configurado de memoria/textura GPU.
- Para resultados grandes: dividir en mosaicos o utilizar otro método de captura de alta resolución comprobado; nunca escalar sin límite una textura pequeña.
- La cámara y la luz deben producir el mismo encuadre y color tanto en la vista previa como en la exportación.
- Usar sRGB, manejo de alfa y corrección de color coherentes con el compositor 2D; comprobar halos en bordes con transparencias.
- La rasterización debe ser **determinista** para una misma versión de motor y los mismos activos, tolerando pequeñas diferencias de hardware.
- Para exportación de vídeo, si se incorpora posteriormente, generar fotogramas del estado actual de cámara; animaciones 3D internas quedan fuera de la primera versión.

## 9. Seguridad de importación

- Tratar `.glb`, `.obj`, `.mtl` y texturas como datos no confiables.
- Detectar formato mediante inspección de contenido y extensión; verificar encabezado, versión y longitudes de GLB.
- Limitar tamaño total del archivo, número de vértices, triángulos, materiales, texturas, nodos y profundidad del árbol; proponer valores iniciales configurables, luego ajustar con pruebas reales.
- Proteger de archivos dañados, geometrías con NaN/infinito, dimensiones extremas y agotamiento de memoria.
- No cargar URLs remotas o rutas arbitrarias referenciadas desde OBJ/MTL; resolver solo dependencias autorizadas por el importador.
- Restringir traversal (`../`), rutas absolutas, enlaces simbólicos y archivos zip bomb en paquetes de dependencias.
- Usar límites de tiempo y memoria en decodificación cuando la plataforma lo permita; evitar bloquear el hilo principal con archivos pesados.
- Ninguna importación debe ejecutar scripts incluidos en modelos ni abrir procesos del sistema.
- Si se produce fallo, el documento debe permanecer íntegro y el error debe comunicar causas y opciones de recuperación.

## 10. Rendimiento

- Importación asíncrona con progreso y cancelación; procesamiento en worker cuando el parser y el entorno lo permitan.
- Caché de recursos por hash y de previews por **asset + estado de instancia + versión de renderer + tamaño de render**.
- Renderizado bajo demanda, sin bucle de animación continuo fuera de la edición.
- Evitar dispositivos y sesiones GPU redundantes; reutilizar recursos `wgpu` y liberar recursos transitorios al cerrar el editor.
- Ajustar automáticamente resolución interactiva en dispositivos lentos, sin reducir calidad del render de exportación.
- Soportar pérdida/recreación de dispositivo GPU y fallback a la última preview válida.
- Ofrecer indicadores claros para modelos de alta complejidad y permitir cancelar si el proceso consume demasiados recursos.

## 11. Accesibilidad e internacionalización

- Etiquetas claras en español/inglés: **Editar 3D**, **Girar**, **Mover**, **Escalar**, **Encuadrar**, **Restablecer**, **Aplicar**, **Cancelar**.
- Atajos documentados, foco visible, controles accesibles desde teclado y alternativa numérica a interacciones de arrastre.
- No depender exclusivamente del color para mostrar el modo activo.
- En dispositivos táctiles, zonas de toque adecuadas y prevención del scroll accidental sobre el editor.
- Las lecturas de ángulos deben indicar grados y la escala su unidad/porcentaje, evitando confundir zoom de cámara con escala del modelo.

## 12. Criterios de aceptación

| ID | Escenario | Resultado exigido |
|---|---|---|
| AC-01 | Importar GLB válido con texturas | El modelo se ve en el lienzo con materiales razonablemente fieles y sin fondo |
| AC-02 | Importar OBJ sin MTL | Se importa la geometría con material neutro y aviso no bloqueante |
| AC-03 | Seleccionar un modelo | Se usan controles normales 2D; no gira por accidente |
| AC-04 | Doble clic / Editar 3D | Se abre editor encapsulado sin cambiar otros elementos |
| AC-05 | Arrastrar en modo Girar | Cambia la orientación 3D y no mueve el elemento 2D |
| AC-06 | Mover/Zoom/Escalar | Cada control realiza una operación diferenciada y predecible |
| AC-07 | Elegir vista frontal/lateral/iso | La orientación cambia de modo repetible |
| AC-08 | Cancelar edición | El elemento recupera exactamente la vista previa anterior |
| AC-09 | Aplicar y deshacer | La modificación se confirma como acción reversible |
| AC-10 | Duplicar el objeto | Las instancias comparten asset base pero pueden tener distintas orientaciones |
| AC-11 | Guardar y reabrir `.gaf` sin archivos originales externos | El modelo sigue disponible y completamente editable en 3D |
| AC-12 | Exportar PNG transparente | Se ve solo el modelo proyectado, sin fondo ni gizmos |
| AC-13 | Exportar JPEG y PDF | Se integra el objeto en la composición respetando capas, tamaños y transparencia según cada formato |
| AC-14 | Exportar a alta resolución | La geometría mantiene calidad según la resolución final; no se reutiliza una miniatura borrosa |
| AC-15 | Modelo inválido o exageradamente pesado | Error controlado; la app no se congela ni corrompe el proyecto |
| AC-16 | Múltiples modelos en el documento | El lienzo no mantiene un contexto WebGL por instancia y conserva fluidez razonable |
| AC-17 | Cerrar/reabrir editor y pérdida de GPU | Última preview válida disponible y opción de regeneración |
| AC-18 | Capas, recorte y rotación 2D | La proyección 3D se comporta como otros elementos gráficos en la composición |

## 13. Plan de entrega

### Fase 1 — MVP imprescindible

1. Modelo de datos, importación `.glb` / `.obj` sin materiales externos complejos, validación y empaquetado en `.gaf`.
2. Previsualización transparente, selección y transformación del elemento desde el lienzo 2D.
3. Overlay 3D con giro intuitivo, zoom, movimiento básico, escala y encuadre.
4. Vistas predefinidas; Aplicar/Cancelar; deshacer/rehacer.
5. Rasterización transparente de alta calidad e integración con exportación PNG/JPEG/PDF.
6. Duplicación independiente, persistencia, limpieza GPU y pruebas de regresión.

### Fase 2 — Compatibilidad y pulido

1. Soporte `.mtl` y texturas locales para OBJ.
2. Compatibilidad con extensiones GLB que requieran decodificadores empaquetados.
3. Ortográfica y controles avanzados X/Y/Z, control de iluminación básica.
4. Accesibilidad ampliada, optimización de escenas complejas y gestión de errores.

### Fase 3 — Futuro, no comprometido

1. Presets de iluminación y sombras opcionales.
2. Biblioteca/catalogación de modelos 3D con miniaturas.
3. Exportar pose o modelo 3D modificado a GLB.
4. Soporte limitado para elegir fotograma de una animación importada.

## 14. Decisiones de diseño que no deben alterarse

1. **Gafira permanece como editor 2D.** El editor 3D solo aparece cuando el usuario lo solicita.
2. **Importar `.glb` y `.obj` no implica editar geometría.** Solo transformar, orientar, encuadrar e iluminar de forma básica.
3. **La representación de lienzo es 2D.** La escena 3D solo se activa al editar o volver a renderizar.
4. **Una imagen 3D renderizada no es vectorial.** El elemento se comporta como gráfico 2D editable y exportable, pero su visualización se rasteriza.
5. **Los originales permanecen dentro de `.gaf`.** El proyecto debe ser portable, independiente de rutas externas.
6. **Separar transformaciones 2D y 3D.** Nunca confundir mover el elemento con desplazar la geometría dentro del encuadre.
7. **Exportación fiel al diseño.** Solo la proyección visible del modelo; guías y UI excluidas.
8. **El rendimiento escala con los elementos.** No abrir un renderizador 3D nativo independiente y continuo por cada objeto.

## 15. Implementación (fase 1)

Decisión del titular: render 3D propio por CPU en lugar de `wgpu` (D-130). Lo que sigue describe lo construido en la rama `feat/100-objetos-3d`.

### 15.1. Crates y capas

| Crate | Capa | Qué hace |
|---|---|---|
| `gafira-model3d` | dominio (fila V5 sin dependencias internas, `clippy.toml` canónico) | Escena (primitivas, materiales, texturas), límites y validación; normalización a la esfera unidad (centro de la caja, radio del vértice más lejano); pose (cuaternión, posición, escala) y ángulos X/Y/Z solo para la interfaz; cámara orbital (giro, elevación, zoom, campo de visión, perspectiva u ortográfica) y vistas rápidas; iluminación Suave, Estudio y Dramática; rasterizador determinista por bandas con supersampling, z-buffer, recorte en el plano cercano, materiales opacos, con máscara y mezclados, texturas de color, metal/rugosidad, oclusión y emisión. Sin hilos ni E/S. |
| `gafira-io-model3d` | adaptador | Lectura de GLB (`gltf`, sin la función `import`: nada fuera del archivo) y OBJ (`tobj`) como datos no confiables: cabecera y longitudes, rangos de cada accesor y vista comprobados antes de leer, extensiones requeridas rechazadas con su nombre (Draco, Meshopt, KTX2…), búferes o imágenes con URI rechazados, límites de vértices, triángulos, materiales, texturas, nodos y profundidad, protección ante pánicos del lector. `ModelDecoder` implementa el método nuevo `ImageDecoder::render_model` del puerto y pinta en paralelo por bandas (`rayon`). |
| `gafira-doc` | dominio | `NodeKind::Model3d(Box<Model3dNode>)` (etiqueta `model3d`): asset del archivo original, formato, pose, cámara e iluminación; validación (valores finitos y en rango, asset presente) y clave de caché `shown()` independiente del tamaño. |
| `gafira-edit` | dominio | `Property::Model3d`: aplicar una edición 3D es un solo paso de deshacer. |
| `gafira-ports`, `gafira-engine`, `gafira-render`, `gafira-io-pptx` | | `model_pixels` y `render_model` en el puerto de exportación; el motor dibuja cada modelo a 4 veces su caja (máx. 4096 px) para exportar y a su caja para vistas previas y miniaturas; el grafo lo compone como una imagen; PowerPoint lo inserta como PNG. |
| `gafira-ui` | adaptador | Módulo `model3d.rs`: servicio de render en un hilo (la petición más reciente de cada elemento gana; el lienzo muestra la última imagen hasta que llega la nueva), sesión de edición, gizmos, cubo de orientación, barra de herramientas, panel «Objeto 3D» y miniaturas de Biblioteca. |

### 15.2. Experiencia

- Importar: «Importar archivo…» (filtro con GLB y OBJ), arrastrar al lienzo, pegar la ruta o desde Biblioteca → pestaña **3D**, que muestra cada modelo importado con una vista previa renderizada, su nombre y su formato. Un archivo inválido no cambia el diseño y muestra la causa; un OBJ sin su MTL entra con material neutro y un aviso.
- En el lienzo el modelo es un elemento más (mover, tamaño, giro 2D, capas, opacidad, duplicar, alinear). Doble clic o «Editar 3D» (barra contextual o panel) abre el editor sobre el propio elemento, sin tiradores 2D:
  - **Girar**: arrastre libre tipo trackball; el aro azul solo gira a los lados y el verde solo arriba o abajo; etiqueta con X/Y/Z durante el arrastre.
  - **Mover**: flechas X (rojo), Y (verde), Z (azul) del marco del modelo para mover en un eje; fuera de ellas, en el plano de la pantalla.
  - **Escalar**: arrastre desde el centro hacia fuera o hacia dentro, con flechas en las esquinas y el porcentaje.
  - **Cámara**: arrastre para rodear e inclinar, con etiquetas «Giro» y «Elevación»; la rueda o el pellizco acercan la cámara en cualquier herramienta, nunca la página.
  - Cubo de orientación arriba a la derecha del lienzo: ejes del modelo vistos por la cámara, nombre de la vista y clic en un extremo de eje para ir a esa vista.
  - Barra inferior: Girar, Mover, Escalar, Cámara, Vista (menú de vistas rápidas), Encuadrar, Restablecer, Cancelar y Aplicar. Tarjeta de ayuda con lo que hace la herramienta y «Intro aplica · Esc cancela».
  - Deshacer y rehacer dentro del editor son de sus propios pasos; Aplicar deja un solo paso en el historial; Cancelar o Esc no cambian nada; elegir otro elemento aplica lo hecho sin preguntar.
- Panel «Objeto 3D» con pestañas **Transformar** (rotación X/Y/Z, tamaño del modelo con tamaños rápidos 50–200 %, posición en el recuadro, restablecer), **Cámara** (vistas rápidas con miniaturas del propio modelo, giro, elevación, distancia, perspectiva u ortográfica, campo de visión, restablecer vista) y **Luz** (Suave, Estudio, Dramática). Fuera del editor los cambios del panel van directos al documento, un paso por gesto.
- Colores de los ejes: tokens nuevos `gizmo.x`, `gizmo.y`, `gizmo.z` en los modos claro y oscuro. Iconos Phosphor nuevos registrados en `assets/LICENSES.toml`.

### 15.3. Criterios de aceptación

| ID | Estado | Evidencia |
|---|---|---|
| AC-01 | cumplido | `DamagedHelmet.glb`, `Avocado.glb`, `capybara.glb`, `phoenix.glb` con sus texturas (revisión visual); `s100_glb_imports_with_its_material` |
| AC-02 | cumplido | `s100_obj_without_mtl_uses_the_neutral_material`; aviso en el lienzo |
| AC-03, AC-04 | cumplido | un clic selecciona en 2D; doble clic abre el editor (revisión en la app) |
| AC-05, AC-06, AC-07 | cumplido | gizmos probados en la app; `s100_view_turns_are_reversible`, `s100_zoom_scales_the_projection`, `s100_quick_views_are_deterministic` |
| AC-08, AC-09 | cumplido | `s100_models_import_edit_and_export`, `s100_the_model3d_is_changed_and_undone`, `s100_the_session_undoes_its_own_steps` |
| AC-10, AC-11 | cumplido | `s100_models_travel_inside_the_gaf_and_share_their_file`; recuperación tras cierre forzado con el modelo |
| AC-12 | cumplido | `s100_render_has_a_transparent_background`, `s100_the_decoder_draws_a_model_on_transparency`, vista previa de exportación en `s100_models_import_edit_and_export` |
| AC-13 | cumplido por diseño | PNG, JPEG, WebP, AVIF, PDF y SVG usan los mismos píxeles del modelo que la vista previa; PowerPoint lo inserta como PNG |
| AC-14 | cumplido | la exportación dibuja a 4 veces la caja (máx. 4096 px) desde el modelo, no desde la miniatura |
| AC-15 | cumplido | `s100_bad_inputs_fail_cleanly`, `s100_external_buffers_are_refused`, `s100_invalid_scenes_are_refused` |
| AC-16 | cumplido | un solo hilo de render para todo el editor; sin contexto gráfico por elemento |
| AC-17 | cumplido | sin GPU que perder (D-130); el lienzo conserva la última imagen válida |
| AC-18 | cumplido | el modelo se compone como una imagen: capas, opacidad, giro 2D y recorte de la página |

### 15.4. Pendiente

- Importación en segundo plano con progreso y cancelación: hoy se lee al importar (el fénix OBJ de 82 MB tarda menos de medio segundo; el GLB de 800 000 triángulos se dibuja en unos 130 ms).
- Fase 2: MTL y texturas de OBJ (sus dependencias dentro del `.gaf`), mapas normales, extensiones GLB con decodificadores empaquetados.
- Vista previa persistida en el `.gaf` para versiones sin 3D: una versión anterior de Gafira no abre un documento con modelos 3D (tipo de nodo desconocido).
- Gestos táctiles propios del editor 3D (pellizco ya acerca la cámara).

## 16. Referencias técnicas

- [glTF 2.0 — Khronos](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
- [wgpu — renderizado GPU nativo Rust](https://wgpu.rs/doc/wgpu/)
- [gltf crate — importación glTF/GLB](https://docs.rs/gltf/)
- [tobj crate — importación OBJ](https://docs.rs/tobj/)
- [glam crate — transformaciones 3D](https://docs.rs/glam/)
- [serde — serialización de datos](https://serde.rs/)
- [rend3 — alternativa a evaluar, comprobar mantenimiento antes de adoptar](https://github.com/BVE-Reborn/rend3)

---

**Resultado esperado:** el usuario coloca un personaje u objeto 3D en un diseño, hace doble clic para girarlo y encontrar el ángulo deseado, pulsa «Aplicar», y continúa editando el documento como si hubiera insertado una imagen o ilustración convencional, sin perder la posibilidad de volver al modelo tridimensional.
