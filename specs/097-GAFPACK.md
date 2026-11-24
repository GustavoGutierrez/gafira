# 097 — Paquetes de recursos (`.gafpack`)

> **Estado:** in-progress — 2026-11-23 — fases 1 y 2 implementadas y probadas en Xephyr (D-128); pendiente la fase 3 (§17) y la prueba del titular.

# Gafira Asset Packages (`.gafpack`)

**Especificación funcional y técnica — versión 1.1 (propuesta)**  
**Estado:** Diseño para implementación  
**Producto:** Gafira — aplicación de escritorio desarrollada en Rust  
**Extensión oficial:** `.gafpack`  
**Tipo MIME propuesto:** `application/vnd.gafira.gafpack+zip`  
**Versión del formato:** `1.1`

---

## 1. Resumen

`.gafpack` es el formato **instalable, portátil y autocontenido** de Gafira para distribuir bibliotecas de recursos gráficos. Un paquete puede contener una o varias categorías de recursos y, tras su instalación, cada elemento debe aparecer **automáticamente en la sección que le corresponda** dentro del panel de recursos de Gafira, con sus metadatos, previsualización y capacidades de uso habituales.

Tipos inicialmente contemplados:

- **Clip art**: imágenes recortadas con transparencia (PNG/WebP) y clip art vectorial (SVG).
- **Ilustraciones**: ilustraciones ráster con transparencia o vectoriales.
- **Iconos**: SVG o imágenes con transparencia.
- **Formas**: formas vectoriales listas para editar o parametrizar cuando el motor lo permita.
- **Marcos**: elementos decorativos o marcos que reciben imágenes.
- **PowerClips / máscaras**: contenedores con una región de recorte en la que se inserta contenido.
- **Collages**: composiciones con múltiples espacios de fotografías/recortes.
- **Templates / plantillas**: diseños reutilizables de página o lienzo, con elementos editables.

Un mismo `.gafpack` puede mezclar categorías. **No debe ser necesario crear una extensión diferente por cada tipo de recurso.**

> **Decisión clave:** la categoría visual donde se muestra un recurso depende de su `type` y `placement`, no del nombre de la carpeta del ZIP ni de su extensión de archivo. Por ejemplo, un PNG transparente declarado como `illustration` se muestra en **Recursos → Ilustraciones**, aunque el archivo esté dentro de `assets/clipart/`.

## 2. Objetivos y alcance

### 2.1 Objetivos

1. Distribuir e instalar colecciones de recursos sin depender de servicios en línea.
2. Identificar autor, paquete, versión, licencias, origen y recursos contenidos.
3. Validar la integridad, compatibilidad y seguridad del paquete antes de instalarlo.
4. Indexar los recursos para búsqueda, filtros, etiquetas, categorías y miniaturas.
5. Integrarlos en los **flujos ya existentes** de inserción y edición de Gafira: arrastrar al lienzo, clic para insertar, transformar, recortar y editar según el tipo.
6. Permitir instalación, actualización, desinstalación y detección de recursos duplicados.
7. Mantener la portabilidad de los documentos `.gaf`, aunque el paquete sea eliminado.
8. Permitir creación y exportación de paquetes desde Gafira en una fase posterior.
9. Exigir una **miniatura general** (`thumb.webp`) y un manifiesto con **autor, tipo principal de paquete, nombre y descripción en español e inglés**, de modo que Gafira pueda identificar y mostrar el paquete antes de instalarlo.

### 2.2 Fuera de alcance de v1

- Ejecutar scripts, binarios, macros o plugins de terceros dentro de `.gafpack`.
- Instalar tipografías en el sistema operativo.
- Sincronizar automáticamente catálogos en la nube.
- Crear un marketplace o sistema de pagos; el formato no debe impedirlo en el futuro.
- Convertir cualquier SVG arbitrario en un objeto plenamente editable cuando el motor todavía no lo soporte.

## 3. Arquitectura del formato

### 3.1 Contenedor

- Un `.gafpack` es un archivo **ZIP estándar** con extensión personalizada.
- Los archivos `manifest.json` y `thumb.webp` son **obligatorios** y están situados en la raíz del ZIP.
- Las rutas referenciadas por el manifiesto son **relativas a la raíz**.
- Los recursos originales se almacenan sin dependencia de archivos externos.
- **Debe contener `thumb.webp`**, miniatura de identidad del paquete; además puede incluir vistas previas individuales WebP/PNG para evitar renderizar todos los originales en listados.
- No se permiten enlaces simbólicos, rutas absolutas ni rutas que salgan del paquete.
- Compresión de entradas: `store` o `deflate` en v1; evitar algoritmos exóticos para facilitar compatibilidad.
- Se reserva `META-INF/` para posibles firmas o información de verificación futura; en v1 no implica confianza de autor.

### 3.2 Estructura sugerida

```text
navidad-clipart.gafpack
├── manifest.json                     # Obligatorio
├── thumb.webp                        # OBLIGATORIO: miniatura general del paquete
├── assets/
│   ├── illustrations/
│   │   ├── santa.png                  # PNG RGBA transparente
│   │   ├── snowman.webp               # WebP con canal alpha
│   │   └── wreath.svg                 # Ilustración vectorial
│   ├── icons/
│   │   └── gift.svg
│   ├── shapes/
│   │   └── star.svg
│   ├── frames/
│   │   └── festive-frame.svg
│   ├── masks/
│   │   └── heart-mask.svg
│   ├── collages/
│   │   └── two-photos.json
│   └── templates/
│       └── greeting-card.json
├── previews/
│   ├── santa.webp
│   ├── snowman.webp
│   ├── wreath.webp
│   ├── festive-frame.webp
│   ├── two-photos.webp
│   └── greeting-card.webp
└── licenses/
    └── LICENSE.txt
```

La organización física es libre; el manifiesto es la **fuente de verdad** de los elementos incluidos y su comportamiento.

## 4. Esquema del manifiesto

El manifiesto debe validarse mediante un esquema formal JSON Schema, versionado por `formatVersion`. Los nombres de claves están en inglés por interoperabilidad; la interfaz de Gafira los traduce.

### 4.1 Campos del paquete

El paquete debe identificarse de forma completa **sin abrir ni interpretar los recursos gráficos originales**. Los metadatos se leen desde `manifest.json` y la miniatura desde `thumb.webp`.

| Campo | Obligatorio | Significado |
|---|---|---|
| `format` | Sí | Literal `gafpack`. |
| `formatVersion` | Sí | Versión de esquema `1.1`. |
| `id` | Sí | Identificador persistente y único, p. ej. `com.gafira.christmas.illustrations`. |
| `version` | Sí | Versión SemVer (`1.0.0`). |
| `packageType` | Sí | Tipo principal del paquete, según §4.1.1. |
| `categories` | Sí | Categorías presentes, derivables y comprobables contra `assets`. |
| `name` | Sí | Objeto localizado con claves `es` y `en`, ambas cadenas no vacías. |
| `description` | Sí | Objeto localizado con claves `es` y `en`, ambas cadenas no vacías. |
| `author` | Sí | Objeto con `name` obligatorio; `id`, `url` y `email` opcionales. Son datos declarados, no identidad verificada. |
| `thumbnail` | Sí | Ruta literal `thumb.webp` a la vista previa general obligatoria. |
| `license` | Sí | Identificador SPDX o `LicenseRef-*` acompañado de su texto. |
| `licenseFile` | Condicional | Obligatorio cuando `license` sea `LicenseRef-*`. |
| `minGafiraVersion` | Sí | Versión mínima compatible de Gafira. |
| `maxGafiraVersion` | No | Solo si hay incompatibilidad conocida. |
| `tags` | No | Etiquetas multilingües o términos de indexación. |
| `assets` | Sí | Lista no vacía de recursos con identificadores únicos. |
| `createdAt`, `updatedAt` | No | Marcas de tiempo ISO 8601, informativas, no confiables para seguridad. |
| `homepage`, `copyright` | No | Página del paquete y aviso de copyright. |

#### 4.1.1 Tipo principal de paquete (`packageType`)

Es **obligatorio** y describe la identidad general de la colección, no el tipo de cada recurso. Valores admitidos en v1.1:

| Valor interno | Español | Inglés | Destino esperado |
|---|---|---|---|
| `clipart` | Clip art | Clip art | Ilustraciones |
| `illustrations` | Ilustraciones | Illustrations | Ilustraciones |
| `icons` | Iconos | Icons | Iconos |
| `shapes` | Formas | Shapes | Formas |
| `frames` | Marcos | Frames | Marcos |
| `powerclips` | PowerClips / máscaras | PowerClips / masks | PowerClips |
| `collages` | Collages | Collages | Collages |
| `templates` | Plantillas | Templates | Plantillas |
| `mixed` | Mixto | Mixed | Cada recurso según su `type`/`placement` |

**Reglas:** `packageType` es una clasificación para mostrar, buscar y filtrar **paquetes**. No sobrescribe la categoría individual de los elementos. Si hay varias familias sustanciales de recursos, el creador puede elegir `mixed` y declarar todas en `categories`; si el tipo principal es específico, las categorías y los recursos deben incluir esa familia (con `clipart` alojado en `illustrations`). `categories` no puede inventar categorías ausentes.

#### 4.1.2 Nombre y descripción bilingües

`name` y `description` son objetos de traducción, no cadenas simples. Deben incluir **ambas variantes** `es` y `en` en UTF-8. Se admite incorporar otros idiomas mediante códigos BCP 47 en una futura versión, sin desplazar `es` ni `en`.

- El instalador selecciona el idioma de la interfaz (`es` o `en`) y muestra su traducción correspondiente.
- No se admiten traducciones vacías, solo espacios o descripciones deliberadamente falsas para completar campos.
- Sugerencia de límites de validación: nombre de 1 a 100 caracteres Unicode y descripción de 10 a 1000 caracteres, después de eliminar espacios exteriores.
- Los nombres y las descripciones localizadas se indexan para búsquedas en cualquiera de los dos idiomas.
- Los recursos individuales pueden continuar con nombres simples en v1.1 y opcionalmente incorporar traducciones, sin obligar a traducir cientos de nombres en un paquete.

#### 4.1.3 Miniatura obligatoria del paquete (`thumb.webp`)

**Contrato de formato:** todo `.gafpack` v1.1 contiene `thumb.webp` en la raíz y el manifiesto declara `"thumbnail": "thumb.webp"`. Este recurso representa **el paquete completo**, no un elemento aislado.

- **Formato:** WebP estático válido, decodificable por el motor gráfico de Gafira. Se prohíbe WebP animado en la miniatura general.
- **Dimensiones recomendadas:** 1024 × 1024 px (proporción 1:1) para catálogo, diálogo de instalación y gestor de paquetes.
- **Dimensiones aceptadas sugeridas:** entre 256 × 256 y 2048 × 2048 px, con proporción 1:1; el creador debe producir una versión cuadrada en vez de deformar la imagen.
- **Tamaño máximo sugerido:** 2 MiB comprimidos, más límites de píxeles decodificados establecidos por el importador.
- **Contenido visual:** collage representativo o montaje de varias piezas; no requiere contener todas las piezas ni garantiza su existencia: prevalece el inventario validado.
- **Transparencia:** permitida, pero la UI debe presentar fondo de contraste o tablero ajedrezado sin modificar los píxeles originales.
- **Seguridad:** comprobar firma/decodificación real WebP, dimensiones y límites de memoria; no confiar únicamente en nombre o extensión.
- **Uso:** mostrar durante la inspección **antes de instalar**, en la biblioteca de paquetes, búsqueda de colecciones, detalles y confirmación de actualización/desinstalación.
- **Fallo:** ausencia, ruta distinta, imagen corrupta, animada o fuera de límites implica paquete **inválido**: no instalar. La creación de paquetes puede generar automáticamente `thumb.webp` antes de exportar, pero el instalador no debe suplir su ausencia silenciosamente.

La miniatura general es **distinta** de `assets[].preview`: esta última muestra cada pieza y sigue siendo opcional si el motor puede generar una vista previa segura.

### 4.2 Campos comunes de cada recurso

| Campo | Obligatorio | Significado |
|---|---|---|
| `id` | Sí | Identificador único **dentro del paquete**. |
| `type` | Sí | Tipo semántico del recurso, según tabla de §5. |
| `name` | Sí | Nombre mostrado en Gafira. |
| `file` | Sí, salvo compuestos | Ruta del archivo fuente; recursos compuestos pueden usar un descriptor `definition`. |
| `definition` | En compuestos | Ruta a descriptor Gafira JSON validado. |
| `preview` | Recomendado | Miniatura para bibliotecas y búsqueda. |
| `placement` | No | Sección destino explícita, validada contra los destinos permitidos del tipo. |
| `tags` | Recomendado | Palabras clave de búsqueda normalizadas. |
| `keywords` | No | Sinónimos, útiles para búsqueda ES/EN. |
| `width`, `height` | No | Dimensiones nominales; el importador las verifica y puede inferirlas. |
| `unit` | No | Unidad de medidas: `px` por defecto; para SVG puede derivarse de `viewBox`. |
| `editable` | No | Solicitud declarativa de capacidades; Gafira determina las realmente disponibles. |
| `license` | No | Licencia específica si difiere de la del paquete. |
| `hash` | Recomendado | SHA-256 hexadecimal del contenido binario para detectar corrupción. |

Los campos adicionales desconocidos se ignoran solamente cuando el esquema permita extensiones `x-*`; ningún dato arbitrario debe activar ejecución o acceso al sistema.

### 4.3 Ejemplo completo: paquete de clip art transparente con metadatos bilingües

El archivo ZIP debe incluir al menos `manifest.json`, `thumb.webp`, `licenses/LICENSE.txt` y los archivos de recursos declarados.

```json
{
  "format": "gafpack",
  "formatVersion": "1.1",
  "id": "com.gafira.collections.christmas-clipart",
  "version": "1.0.0",
  "packageType": "clipart",
  "categories": ["illustrations"],
  "name": {
    "es": "Clip art navideño",
    "en": "Christmas Clip Art"
  },
  "description": {
    "es": "Colección de ilustraciones navideñas PNG y WebP transparentes, junto con vectores SVG.",
    "en": "Collection of transparent Christmas PNG and WebP illustrations, plus SVG vectors."
  },
  "author": {
    "name": "Gafira Studio",
    "url": "https://gafira.com"
  },
  "thumbnail": "thumb.webp",
  "license": "LicenseRef-Gafira-Christmas-Pack",
  "licenseFile": "licenses/LICENSE.txt",
  "minGafiraVersion": "1.0.0",
  "tags": ["navidad", "christmas", "clipart", "transparent"],
  "assets": [
    {
      "id": "santa-transparent",
      "type": "clipart",
      "name": "Papá Noel",
      "file": "assets/illustrations/santa.png",
      "preview": "previews/santa.webp",
      "placement": "illustrations",
      "tags": ["navidad", "personaje"],
      "keywords": ["santa", "christmas", "transparent"]
    },
    {
      "id": "snowman-transparent",
      "type": "illustration",
      "name": "Muñeco de nieve",
      "file": "assets/illustrations/snowman.webp",
      "preview": "previews/snowman.webp",
      "placement": "illustrations"
    },
    {
      "id": "wreath-vector",
      "type": "illustration",
      "name": "Corona navideña vectorial",
      "file": "assets/illustrations/wreath.svg",
      "preview": "previews/wreath.webp",
      "placement": "illustrations"
    }
  ]
}
```

La validez del ejemplo exige la existencia física y decodificación de `thumb.webp` y de todos los archivos indicados. `LicenseRef-Gafira-Christmas-Pack` exige texto de licencia real, no inventado. `author` es una atribución **declarada**: no demuestra identidad ni derechos de distribución.

## 5. Tipos de recursos y ubicación en la UI

La asignación por defecto debe ser determinista y compatible con el panel real de Gafira. Los nombres visibles pueden variar por idioma; el código interno usa identificadores estables.

| `type` | Archivos / descriptores admitidos | Destino predeterminado | Al insertarlo |
|---|---|---|---|
| `clipart` | PNG RGBA, WebP alfa, SVG | **Recursos → Ilustraciones** | Objeto imagen o vectorial sobre el lienzo. |
| `illustration` | PNG, WebP, SVG | **Recursos → Ilustraciones** | Objeto imagen/vector conservando transparencia. |
| `icon` | SVG, PNG, WebP | **Recursos → Iconos** | Objeto escalable; recoloreable si el motor vectorial lo permite. |
| `shape` | SVG o definición paramétrica compatible | **Recursos → Formas** | Forma o vector editable según soporte. |
| `frame` | SVG/PNG decorativo o descriptor de marco | **Recursos → Marcos** | Marco decorativo o marco con ranura para foto, según definición. |
| `powerclip` | SVG de máscara + descriptor JSON | **Recursos → PowerClips / Máscaras** | Contenedor recortable editable. |
| `collage` | Descriptor JSON + recursos asociados | **Recursos → Collages** | Composición con múltiples zonas de imagen. |
| `template` | Documento/escena JSON Gafira (esquema versionado) | **Plantillas** | Crea documento o inserta diseño según intención del usuario. |

Si la navegación actual utiliza otras etiquetas, el implementador debe **mapear estos identificadores a las secciones existentes**, sin duplicar categorías innecesariamente. Si una sección todavía no existe, crearla o habilitarla dentro del apartado Recursos con criterio consistente.

### 5.1 Reglas de `placement`

- `placement` es opcional; si falta, se usa el destino de la tabla.
- Solo se admiten combinaciones `type` + `placement` permitidas. Ejemplo válido: `clipart` → `illustrations`.
- Un recurso puede contar con varias etiquetas, **pero un destino principal**. Si el producto requiere aparecer en varias vistas, aplicar filtros/colecciones o vistas virtuales sin duplicar físicamente el archivo.
- Si una categoría futura no está soportada por la versión instalada, se debe informar de incompatibilidad o rechazar el paquete según la política versionada; no ubicar elementos silenciosamente en una sección incorrecta.

### 5.2 Comportamiento específico del clip art transparente

Caso prioritario: el usuario tiene una colección de PNG o WebP con fondo transparente y quiere crear un `.gafpack` para utilizarla desde **Ilustraciones**.

1. El creador del paquete agrega las imágenes y asigna `type: "clipart"` o `type: "illustration"` y `placement: "illustrations"`.
2. Durante importación se comprueba que las imágenes se pueden decodificar. Para una colección declarada como **clip art transparente**, se verifica que tienen canal alfa y, opcionalmente, que existe algún píxel con alfa inferior a 255; si la colección exige transparencia, los recursos opacos generan advertencia o error conforme a su configuración.
3. Gafira instala e indexa cada imagen como recurso independiente, con miniatura y fondo de tablero de ajedrez o fondo neutro para evidenciar transparencia.
4. En **Recursos → Ilustraciones**, la persona puede explorar por colección, buscar por nombre/etiquetas y arrastrar una imagen al lienzo.
5. La imagen se inserta con su **transparencia real**, sin agregar fondo blanco, y conserva calidad original y proporción. Se representa como objeto ráster si es PNG/WebP y como SVG/vector si se admite ese tipo.
6. Los cambios de escala, rotación, posición, opacidad y orden de capas usan las herramientas habituales del editor.
7. La exportación del diseño respeta la transparencia y el resultado visible conforme al formato elegido.

> No confundir “formato PNG” con “imagen transparente”: un PNG puede estar completamente opaco. Debe comprobarse su contenido o marcarse explícitamente como no transparente.

## 6. Definiciones compuestas: marcos, PowerClips, collages y plantillas

No basta con guardar imágenes para todos los tipos. Algunos requieren semántica para funcionar como objetos editables.

### 6.1 Marco decorativo vs. marco con foto

- **Decorativo:** un SVG/PNG insertado como elemento visual independiente.
- **Con foto:** descriptor que establece contenedor, máscara, relación de aspecto y zona de imagen. Al insertarse, el usuario puede arrastrar una foto a la zona y cambiar su recorte sin modificar el adorno.
- La UI debe distinguir visualmente ambos casos, aunque se presenten en **Marcos**.

### 6.2 PowerClip / máscara

Un descriptor define `mask`, geometría de la máscara, comportamiento de ajuste (`cover`/`contain`), posición inicial del contenido y reglas de edición. La máscara debe funcionar dentro del motor de recorte de Gafira; no debe ser una vista previa aplanada.

Ejemplo conceptual de definición `assets/masks/heart-mask.json`:

```json
{
  "schema": "gafira.powerclip",
  "version": 1,
  "mask": "heart-mask.svg",
  "contentFit": "cover",
  "preserveAspectRatio": true,
  "editableContent": true
}
```

Las rutas dentro de descriptores deben resolverse **respecto al descriptor** dentro del paquete y pasar la misma validación de seguridad que las rutas del manifiesto. Este esquema se considera una **interfaz propuesta**, no una API ya existente: debe adaptarse al modelo de escena real de Gafira.

### 6.3 Collages

Un collage es una escena reutilizable con varias ranuras de imágenes (`slots`), cada una con límites geométricos, máscara opcional, orden de capas y ajuste por defecto. Al agregarlo, se inserta un grupo editable; cada ranura puede recibir una foto distinta.

### 6.4 Plantillas

- Se almacenan mediante un descriptor de **escena/documento Gafira versionado**, no mediante código ejecutable.
- Incluyen tamaño/orientación de lienzo, capas, grupos, elementos, recursos dependientes y miniatura.
- Al elegir una plantilla, mostrar **Crear diseño a partir de plantilla** y, si procede, **Insertar en diseño actual**.
- Deben reutilizar el serializador y el motor de escenas de `.gaf` cuando sea posible, evitando dos representaciones incompatibles.
- Las fuentes externas usadas por una plantilla deben declararse como dependencias, resolverse con alternativas o incrustarse solo si la licencia permite esa distribución; nunca instalar fuentes silenciosamente.

## 7. Flujo de creación de un paquete

### 7.1 Empaquetador (UI posterior y CLI/servicio inicial)

1. Seleccionar **Crear paquete de recursos**.
2. Escribir **nombre y descripción en español e inglés**, identificador estable, **tipo principal de paquete**, autor, licencia y versión.
3. Agregar archivos individuales o una carpeta; detectar formatos y sugerir tipos automáticamente.
4. Permitir asignación de `type`, categoría, nombre y etiquetas por lote (especialmente para colecciones PNG).
5. **Generar o solicitar `thumb.webp` obligatorio** (imagen cuadrada representativa) y generar miniaturas individuales cuando convenga.
6. Ejecutar validaciones locales y mostrar problemas corregibles.
7. Empaquetar los originales, metadatos, previews y licencia.
8. Guardar `<nombre>.gafpack` y permitir abrirlo para inspección antes de compartirlo.

**Regla de automatización:** importar una carpeta de PNG/WebP recortados no debe exigir registrar manualmente cada imagen en JSON. El empaquetador debe generar el inventario desde los archivos y permitir correcciones masivas.

### 7.2 Asistente recomendado de creación

Campos obligatorios iniciales: **Nombre (ES/EN)**, **Descripción (ES/EN)**, **Tipo de paquete**, **Archivos**, **Autor**, **Licencia** y **Miniatura del paquete**. El asistente puede proponer traducciones y generar la miniatura como composición visual, pero el usuario puede revisarlas y corregirlas antes de exportar. El `id`, la versión y el inventario pueden inferirse inicialmente y editarse en ajustes avanzados.

## 8. Flujo de instalación (UX)

### 8.1 Entradas soportadas

- Doble clic sobre `.gafpack` mediante asociación de archivos del sistema operativo.
- **Archivo → Instalar paquete…** en Gafira.
- Arrastrar y soltar `.gafpack` en la ventana.
- **Recursos → Paquetes instalados → Instalar**.

### 8.2 Secuencia

1. Abrir el archivo **sin instalarlo inmediatamente**.
2. Leer índice ZIP y `manifest.json` dentro de límites de seguridad.
3. Validar manifiesto (incluidos `packageType`, autor, `name.es/en`, `description.es/en`), presencia y decodificación de `thumb.webp`, rutas, hashes disponibles, formatos, tamaños, licencia y compatibilidad.
4. Mostrar pantalla de confirmación con **`thumb.webp`**, nombre y descripción localizados, tipo principal traducido, autor **declarado (no verificado)**, versión, licencia, número de recursos y desglose por categoría.
5. Advertir si el paquete proviene de origen desconocido, si sobrescribe una versión instalada o si contiene elementos no compatibles.
6. Confirmar instalación y extraer a área temporal confinada.
7. Validar todos los recursos y descriptores; generar previews faltantes cuando sea seguro y soportado.
8. Registrar y publicar la instalación **de forma atómica**: si falla, conservar la versión anterior y limpiar archivos temporales.
9. Actualizar el índice de búsqueda y las bibliotecas visibles sin reiniciar Gafira.
10. Mostrar resultado: **Paquete instalado**, cantidad por categoría y acción **Ver recursos**.

### 8.3 Gestión de paquetes

En **Configuración / Recursos → Paquetes instalados** mostrar: **miniatura `thumb.webp`, nombre y descripción ES/EN según idioma activo, tipo principal**, autor declarado, versión, licencia, categorías, cantidad, tamaño, fecha de instalación, estado y origen si está disponible.

Acciones: **Ver recursos**, **Actualizar desde archivo**, **Desinstalar**, **Ver detalles**, **Abrir licencia** y **Reparar/Reindexar**.

- **Mismo `id` y misma versión:** no duplicar; mostrar instalado y ofrecer reparación/reinstalación.
- **Mismo `id`, nueva versión:** actualizar de forma transaccional preservando datos del catálogo y referencias estables.
- **Mismo `id`, distinta procedencia/autor declarado:** avisar y tratar como potencial sustitución no confiable; no asumir que el texto del autor acredita propiedad.
- **Desinstalación:** eliminar únicamente archivos administrados por ese paquete tras confirmar; los documentos creados deben conservar los recursos que utilizan.

## 9. Validación de seguridad e integridad

### 9.1 Validación de contenedor

**Rechazar** el paquete cuando:

- ZIP dañado, cifrado o con formato no admitido.
- Falta `manifest.json` o está mal formado.
- La estructura incumple `formatVersion` o versiones compatibles.
- Existe un nombre de entrada absoluto, con `..`, separadores ambiguos, rutas duplicadas normalizadas, enlaces simbólicos o entradas que sobrescriben archivos fuera del directorio permitido.
- Se superan límites configurables: tamaño comprimido, tamaño total descomprimido, número de archivos, tamaño por recurso, profundidad de rutas o tasa de expansión ZIP (ZIP bomb).
- Hay descriptores que referencian rutas inexistentes o externas.
- Hay recursos con tipos incompatibles o formatos prohibidos.
- Un hash declarado no coincide con el contenido real.

Todos los límites deben fijarse como constantes configurables y probarse con casos adversariales; los valores exactos se calibrarán con colecciones reales para no rechazar paquetes legítimos.

### 9.2 Validación de recursos

- **PNG/WebP:** decodificar con bibliotecas seguras, validar dimensiones máximas, evitar agotamiento de memoria y comprobar canal alfa cuando se declare requerido.
- **SVG:** permitir solo el subconjunto soportado por el renderizador: rechazar o neutralizar scripts, entidades externas, `foreignObject`, imágenes remotas, URLs externas, cargas de recursos externos y funcionalidades activas. No confiar en la extensión; inspeccionar contenido.
- **JSON de escena:** validarlo por esquema y referencias; impedir ciclos ilegales, nodos excesivos y propiedades no soportadas.
- **Previews:** tratar imágenes y miniaturas como archivos no confiables, aplicando límites de decodificación.
- **Licencias:** validar presencia de archivos citados; **no** interpretar una licencia declarada como prueba de autorización de distribución.

### 9.3 Integridad y confianza

- Calcular SHA-256 de cada recurso durante importación; verificar los hashes presentes en el manifiesto.
- Calcular hash del paquete completo para diagnóstico y detección de cambios; registrar origen cuando se conozca.
- Los hashes detectan cambios, **pero no autentican al autor**. Una firma criptográfica verificable y un almacén de publicadores confiables se dejan para una versión posterior.
- Prohibir ejecución de código y scripts; `.gafpack` contiene exclusivamente datos gráficos y metadatos.

## 10. Instalación física y catálogo local en Rust

### 10.1 Almacenamiento

Usar un directorio de datos **por usuario**, obtenido mediante API multiplataforma (`directories` o equivalente), nunca una ruta rígida.

```text
<GAFIRA_USER_DATA>/
├── packages/
│   └── com.gafira.collections.christmas-clipart/
│       └── 1.0.0/
│           ├── manifest.json
│           ├── assets/...
│           └── previews/...
├── cache/
│   └── thumbnails/...
└── catalog.sqlite
```

Si el diseño de Gafira prefiere contenido deduplicado por hash, almacenar blobs por contenido y mantener una tabla de referencias; **no es requisito de v1**.

### 10.2 Índice local

Usar SQLite (si ya existe en el proyecto, reutilizar su infraestructura) para consultas de baja latencia:

- `packages`: `package_id`, `version`, `name`, `author`, `license`, `path`, `source`, `installed_at`, `hash`, `status`.
- `assets`: `package_id`, `asset_id`, `type`, `placement`, `name`, `file_path`, `preview_path`, `width`, `height`, `has_alpha`, `mime`, `hash`.
- `asset_tags`: asociación recurso-etiqueta.
- `assets_fts`: índice FTS5 de nombres, descripción y etiquetas (opcional según índice de búsqueda existente).

Clave lógica de recurso: `package_id` + `asset_id`. En una actualización, el `asset_id` se mantiene siempre que represente el mismo recurso.

**Separación de responsabilidades:** archivos del paquete = fuente de contenido; SQLite = índice reconstruible. No modificar originales instalados desde las operaciones normales de diseño.

### 10.3 Módulos Rust sugeridos

```text
crates/ o src/
└── asset_packages/
    ├── manifest.rs          # Modelos serde + versionado
    ├── validator.rs         # ZIP, esquema, archivos y límites
    ├── installer.rs         # Instalación atómica / rollback
    ├── registry.rs          # SQLite e indexación
    ├── previews.rs          # Generación / caché de miniaturas
    ├── resolver.rs          # Resuelve asset -> objeto de lienzo
    ├── exporter.rs          # Creación de .gafpack
    └── errors.rs            # Errores tipados y localización
```

Bibliotecas a evaluar según el stack **ya presente**: `zip`, `serde`, `serde_json`, `semver`, `sha2`, `image`, `rusqlite` (si ya es dependencia) y `directories`; para SVG, preferir un parser/renderizador seguro alineado con el motor de Gafira, por ejemplo `usvg`/`resvg` si encajan.

No introducir un segundo motor gráfico únicamente para `.gafpack`: reutilizar importadores de imagen, SVG, escenas, recortes y caché del editor.

## 11. Inserción en el lienzo y portabilidad del proyecto

Al seleccionar un recurso de la biblioteca:

1. Resolver `package_id` + `asset_id` en el registro.
2. Abrir únicamente recursos previamente validados y/o revalidarlos si hubo cambios en disco.
3. Convertirlo al **mismo tipo de nodo de escena** que genera Gafira mediante importación normal.
4. Insertarlo con la lógica estándar de posición, dimensiones, selección, deshacer/rehacer y propiedades.
5. Guardar opcionalmente procedencia (`sourcePackageId`, `sourceAssetId`, `sourceVersion`) para atribución y trazabilidad.

**Regla de portabilidad:** los documentos `.gaf` no deben depender de que un `.gafpack` siga instalado. Al guardar un proyecto, el recurso utilizado se incorpora/serializa con el mecanismo normal de assets de `.gaf` (o se conserva en un almacén de recursos embebidos de ese documento). Una desinstalación jamás debe romper un diseño existente.

Si la edición modifica un elemento de biblioteca, la instancia del documento se considera independiente; **no se sobrescribe** el recurso fuente del paquete.

## 12. Compatibilidad y evolución del formato

- `formatVersion` gobierna la versión del contrato de `.gafpack`.
- `minGafiraVersion` se usa para compatibilidad funcional con el programa.
- Un esquema JSON Schema debe incluir restricciones de tipos, campos obligatorios, enumeraciones y reglas condicionales por `type`.
- Los cambios compatibles deben preservar manifestaciones antiguas; los incompatibles requieren nueva versión principal del formato.
- El validador debe entregar errores accionables: archivo, recurso, regla violada y propuesta de corrección.
- Mantener los IDs estables y garantizar que los recursos instalados aparecen en su sección aun cuando cambien nombres o rutas internas entre versiones del paquete.

## 13. Interfaz y experiencia de uso

### 13.1 Biblioteca de Recursos

- Filtros **Todos / Ilustraciones / Iconos / Formas / Marcos / PowerClips / Collages / Plantillas** según la arquitectura actual.
- Filtro **Origen: Integrados / Instalados / Todos**.
- Filtro por **Colección/paquete**, **tipo principal de paquete**, autor y etiquetas.
- Cada colección instalada expone su **`thumb.webp`** en la ficha/listado de paquetes y su título/descripción en el idioma elegido.
- Búsqueda por nombre, sinónimos y etiquetas, con soporte ES/EN cuando existan traducciones.
- Miniaturas con proporción adecuada, especialmente iconos, marcos y PNG transparentes.
- En imágenes transparentes, fondo cuadriculado opcional en preview; **nunca se rasteriza ese fondo sobre el recurso**.
- Menú contextual: Insertar, Ver información, Ver colección, Ver licencia.

### 13.2 Instalador

Antes de instalar, mostrar un resumen como:

```text
[thumb.webp]  Clip art navideño     versión 1.0.0
              Paquete: Clip art
              Colección de ilustraciones navideñas...
              Gafira Studio · autor declarado

Contenido
  24 Ilustraciones (PNG/WebP/SVG)
  12 Iconos
  6 Marcos

Licencia: ver términos
Compatibilidad: Gafira >= 1.0.0
Estado: paquete válido

[Cancelar]                         [Instalar paquete]
```

Los conteos deben calcularse a partir del manifiesto validado, no confiar en valores escritos manualmente en él.

## 14. API interna sugerida

API conceptual del subsistema, no contrato obligatorio de módulos existentes:

```rust
pub trait AssetPackageService {
    fn inspect(&self, file: &std::path::Path) -> Result<PackageInspection, PackageError>;
    fn install(&self, file: &std::path::Path) -> Result<InstalledPackage, PackageError>;
    fn uninstall(&self, package_id: &str) -> Result<(), PackageError>;
    fn list_packages(&self) -> Result<Vec<InstalledPackage>, PackageError>;
    fn query_assets(&self, filter: AssetFilter) -> Result<Vec<AssetSummary>, PackageError>;
    fn resolve_asset(&self, package_id: &str, asset_id: &str) -> Result<ResolvedAsset, PackageError>;
}
```

Para UI, comunicar operaciones y progreso mediante los mecanismos propios de la arquitectura actual de escritorio Rust (eventos/comandos/modelos), sin asumir Electron, JavaScript, Three.js ni servicios web.

## 15. Criterios de aceptación

### A. Paquete de clip art transparente

**Dado** un `.gafpack` con 50 PNG RGBA, WebP con alfa y un SVG, declarados como `clipart`/`illustration` y destinados a `illustrations`, **cuando** el usuario lo instala:

- Los 51 elementos aparecen en **Recursos → Ilustraciones** dentro de su colección.
- Cada recurso tiene miniatura y es localizable por nombre y etiquetas.
- PNG/WebP se insertan sin fondo blanco y con transparencia conservada.
- SVG se muestra e inserta según el soporte vectorial real de Gafira.
- Rotación, escala, posición, capas y deshacer/rehacer funcionan como con cualquier otro recurso importado.
- Al guardar y abrir un `.gaf` en otra instalación sin ese paquete, las instancias usadas se siguen visualizando.

### B. Identidad, miniatura y traducción del paquete

**Dado** un paquete v1.1 válido, **cuando** se inspecciona o aparece en la biblioteca, **entonces**:

- La UI muestra `thumb.webp` **antes de instalarlo** y tras su instalación.
- Se muestran nombre y descripción en **español** cuando el idioma de Gafira es español y en **inglés** cuando es inglés.
- Aparecen el autor declarado y el tipo principal traducido, tanto en inspección como en detalles.
- Cambiar el idioma de Gafira actualiza las etiquetas localizadas sin reinstalar.
- No se confunde `packageType` con la clasificación de cada elemento del inventario.

### C. Paquete mixto

**Dado** un paquete con iconos, marcos, formas, collages y plantillas, **cuando** se instala, **entonces** cada elemento aparece exclusivamente en su sección primaria correspondiente y su acción de inserción conserva la semántica del objeto.

### D. Validación y errores

- Un ZIP con `../` o rutas absolutas se rechaza sin escribir fuera del almacenamiento permitido.
- Un recurso declarado pero inexistente genera error identificable.
- La ausencia, corrupción, animación o dimensión inválida de `thumb.webp` bloquea la instalación.
- Se rechaza el manifiesto sin `packageType`, `author.name`, `name.es`, `name.en`, `description.es` o `description.en` válidos.
- Se rechaza un `packageType` desconocido o `categories` que no corresponda al inventario real.
- Un SVG activo o que depende de recursos remotos no accede a red ni ejecuta contenido.
- Un paquete incompatible no se instala parcialmente.
- Un recurso con hash incorrecto se rechaza.
- El instalador puede revertir una instalación fallida y conservar una versión anterior funcional.
- Instalar el mismo ID y versión no crea duplicados.
- Desinstalar un paquete no elimina recursos embebidos dentro de proyectos `.gaf`.

### E. Rendimiento

- La lista inicial usa miniaturas de caché sin decodificar todos los originales al abrir Gafira.
- La indexación permite paginar y filtrar colecciones grandes.
- La interfaz permanece responsiva durante extracción, comprobación y generación de miniaturas mediante tareas de fondo controladas **dentro de la aplicación**.
- Las dimensiones, cantidades y tamaños máximos están restringidos para evitar consumo excesivo de memoria y disco.

## 16. Pruebas mínimas

| Grupo | Casos |
|---|---|
| Formato | ZIP correcto, manifest incompleto, versión desconocida, rutas relativas. |
| Seguridad | ZIP Slip, ZIP bomb, symlinks, rutas duplicadas, SVG con recursos externos o scripts. |
| Recursos | PNG alfa, PNG opaco, WebP alfa, SVG válido/inválido, JSON compuesto incorrecto. |
| UX | Abrir con doble clic, arrastrar paquete, previsualizar, confirmar/cancelar, localizar colección. |
| Registro | Instalar, instalar repetido, actualizar, reparar, desinstalar, reiniciar Gafira. |
| Escenas | Insertar y transformar clip art, editar marco, insertar collage, utilizar plantilla. |
| Portabilidad | Guardar `.gaf`, desinstalar paquete, reabrir el diseño y exportarlo sin pérdida. |
| Rendimiento | Paquetes con cientos o miles de elementos, generación incremental de thumbnails, paginación. |

## 17. Plan de implementación recomendado

**Fase 1 — Núcleo utilizable:** ZIP + manifest, validaciones seguras, instalación/registro SQLite, miniaturas, panel de paquetes, soporte PNG/WebP/SVG como `clipart`, `illustration`, `icon` y `shape`, inserción en el lienzo y portabilidad `.gaf`.

**Fase 2 — Recursos compuestos:** marcos con ranura, PowerClips, collages y plantillas con descriptores de escena versionados, además de pruebas de edición.

**Fase 3 — Autoría y distribución:** creador visual de `.gafpack`, importación masiva y asignación de metadatos, actualización y reparación, firma verificable de publicadores y mecanismos opcionales de catálogo remoto.

Estas fases son una propuesta de entrega, no una reducción del alcance final del formato.

## 18. Decisiones que deben permanecer estables

1. **Extensión:** `.gafpack`.
2. **Formato:** ZIP autocontenido con `manifest.json` versionado.
3. **Identidad:** paquete identificado por `id`; cada recurso por `package_id + asset_id`.
4. **Clasificación:** el manifiesto determina el tipo y la sección de la UI.
5. **Clip art transparente:** PNG/WebP alfa y SVG deben funcionar como ilustraciones insertables.
6. **No ejecución:** solamente datos gráficos y descriptores validados.
7. **Motor nativo Rust:** reutilizar el sistema gráfico y de documentos existente.
8. **Proyectos autónomos:** un `.gaf` sigue funcionando sin el `.gafpack` original.
9. **Instalación transaccional:** nada parcialmente registrado y restauración ante fallo.
10. **Escalabilidad:** admitir colecciones grandes sin bloquear la interfaz ni cargar todas las imágenes simultáneamente.

---

**Resultado esperado:** cualquier diseñador puede obtener un archivo `coleccion.gafpack`, abrirlo en Gafira, revisar qué contiene, instalarlo y encontrar **cada recurso en su lugar natural** para usarlo con las herramientas normales del editor, sin configuraciones técnicas ni trabajo manual posterior.


## 19. Nota de versionado del manifiesto (1.0 → 1.1)

Esta revisión eleva `formatVersion` a **`1.1`** porque ahora son obligatorios `thumb.webp`, `thumbnail`, `packageType`, `categories`, `name.es/en` y `description.es/en`, y cambia la forma de `name` y `description` de cadena a objeto bilingüe.

- **Paquetes 1.1:** deben cumplir estrictamente el contrato nuevo.
- **Paquetes 1.0 ya existentes:** un importador que decida admitirlos debe reconocer explícitamente su esquema anterior y ofrecer una **migración controlada** (selección/generación de miniatura, traducciones y tipo), con confirmación del usuario y reexportación como 1.1. No debe tratarlos como 1.1 ni asumir que sus metadatos faltantes son válidos.
- **Contratos de API:** el modelo de `PackageInspection` y el catálogo local deben exponer `thumbnail`, `package_type`, `localized_name` y `localized_description`; almacenar ambas traducciones para cambios de idioma sin reinstalación.
- **Validador:** mantener casos de prueba positivos y negativos para `thumb.webp`, traducciones obligatorias, autor, tipo de paquete e inventario coherente.

## 20. Implementación — fase 1 (2026-11-23)

Lo que esta entrega cumple, en el orden del plan de §17.

**Hecho**

- **Formato y validación** (`crates/gafira-ui/src/packs.rs`, puro): manifiesto 1.1 con serde (claves camelCase, `x-*` ignoradas); reglas de §4.1 (formato, versión de formato, `id`, SemVer, `packageType`, `name`/`description` ES y EN con sus longitudes, autor, `thumbnail` = `thumb.webp`, licencia y `licenseFile` para `LicenseRef-*`, `minGafiraVersion`/`maxGafiraVersion` contra la versión de Gafira); recursos de §4.2 y §5 (identificador único, tipo, `placement` permitido, archivo seguro y presente, formato PNG/WebP/SVG, hash SHA-256); `categories` igual a las secciones de los recursos y coherente con `packageType`; rutas seguras (§3.1); miniatura WebP estática, cuadrada, de 256 a 2048 px y como mucho 2 MiB (§4.1.3); SVG sin scripts, eventos, entidades, `foreignObject` ni referencias externas (§9.2); imágenes que decodifican dentro de límites; conteos por sección calculados desde los recursos (§13.2).
- **Almacén** (`apps/gafira/src/packs.rs`): lectura del ZIP con límites (10 000 entradas, 512 MiB descomprimidos, 64 MiB por archivo, proporción de bomba ZIP, cifrado, enlaces simbólicos, nombres repetidos); instalación en `packages/<id>/<versión>/` a través de una carpeta temporal y un cambio de nombre, de modo que un fallo deja la versión anterior; misma versión = reinstalar sin duplicar; versión nueva = reemplaza la anterior; desinstalar solo borra la carpeta del paquete. El índice se reconstruye leyendo las carpetas (no hay SQLite en el proyecto; §10.2 lo permite).
- **Inspección antes de instalar** (`pack_dialog.rs`): miniatura, nombre y descripción en el idioma de la interfaz, tipo traducido, versión, autor declarado y no verificado, contenido por sección, licencia, compatibilidad, tamaño y estado (válido, ya instalado o actualización). «Instalar» solo aparece si el paquete es válido; si no, se listan los problemas. Se abre desde **Inicio → Instalar paquete**, con **doble clic** en un `.gafpack` (argumento de la línea de órdenes), **soltando** el archivo en la ventana y desde **Ajustes → Paquetes**.
- **Uso** (`pack_panel.rs`): los recursos aparecen en **Elementos → Ilustraciones** (clip art e ilustraciones), en la categoría nueva **Elementos → Iconos** y en **Elementos → Formas**, agrupados por paquete con su nombre y miniatura, con filtro por paquete y búsqueda por nombre, etiquetas y palabras clave. Clic para insertar, arrastrar para soltar en una página. PNG y WebP entran como imagen con su transparencia; SVG como vectores editables. El recurso se copia al documento, así un `.gaf` no depende del paquete (§11) y no se añade a Biblioteca.
- **Gestión** (Ajustes → Paquetes): lista con miniatura, nombre, descripción, tipo, versión, número de recursos, tamaño y autor declarado; «Desinstalar» con confirmación.
- **Tipo de archivo**: `application/vnd.gafira.gafpack+zip`, extensión `.gafpack`, icono derivado del de `.gaf` (`docs/manuals/marca/assets/gafira-gafpack-icon.svg`, manual de marca 7.14) en Linux (MIME, entrada de escritorio, `install.sh`, `.deb`, AppImage), Windows (instalador y su comprobación en la CI) y macOS (`Info.plist`).

**Pendiente (fase 3 y detalles)**

- Creador de paquetes (§7), migración de paquetes 1.0 (§19), firma de autor (§9.3).
- Filtro por origen integrados/instalados y por autor (§13.1).

**Evidencia**

- `gafira-ui`: `s97_01_*` a `s97_07_*` (manifiesto, identidad, categorías, recursos, versiones, rutas, SVG activo, miniatura, imágenes y hashes) y `s97_11_*` (filtro por sección, paquete y búsqueda). Se escribieron junto con su código en la misma unidad de trabajo.
- `apps/gafira`: `s97_08_*` (inspeccionar sin instalar, instalar, reinstalar sin duplicar), `s97_09_*` (actualizar y desinstalar), `s97_10_*` (ruta fuera del paquete, archivo que falta, hash erróneo, Gafira demasiado antigua y archivo que no es ZIP: nada se escribe).
- `xtask`: los iconos `.gafpack` (hicolor, `.ico`, `.icns`) generados y comprobados con los demás.
- Xephyr: abrir `navidad.gafpack` como argumento muestra la ventana con miniatura y metadatos; «Instalar paquete» lo instala; sus recursos aparecen en Ilustraciones, Iconos y Formas; el PNG transparente se inserta sin fondo y el SVG como grupo vectorial; un paquete con `../../escape.txt` se rechaza sin botón de instalar y sin escribir nada; Ajustes → Paquetes lo lista y lo desinstala tras confirmar.

## 21. Implementación — fase 2 (2026-11-23)

Los recursos compuestos de §6, sobre el modelo real de Gafira (§6.2 lo pide: «adaptarse al modelo de escena real»).

- **Marco** (`frame`): o un archivo decorativo (PNG, WebP o SVG), que se inserta como imagen o vectores, o un descriptor `{"schema":"gafira.frame","version":1,"mask":"x.svg","contentFit":"cover|contain"}`, que se inserta como **marco de Gafira** de la forma de la máscara: recibe una foto arrastrada y su recorte se edita sin tocar la forma.
- **PowerClip** (`powerclip`): descriptor `gafira.powerclip` con la misma forma. En Gafira un marco ya es un contenedor que recorta su contenido, así que se inserta igual, con el ajuste de `contentFit` (`cover` = Rellenar, `contain` = Encajar). Marcos y PowerClips se muestran juntos en **Elementos → Marcos** (§5: se mapean a la sección existente).
- **Collage** (`collage`): descriptor `{"schema":"gafira.collage","version":1,"gap":0.012,"slots":[{"x":0,"y":0,"w":0.5,"h":1},…]}` con 1 a 24 huecos en fracciones de la página. Se inserta como los collages de Gafira: un grupo de marcos que ocupa la página. Se muestra en **Elementos → Foto collage**, dibujando sus huecos si no trae vista previa.
- **Plantilla** (`template`): un `.gaf` de plantilla dentro del paquete (§6.4: el mismo serializador). Se valida al instalar con el lector de plantillas de Gafira y aparece en **Plantillas** (Inicio y editor) con el nombre del manifiesto; se encuentra también por el nombre y el id del paquete. Al instalar o desinstalar, Plantillas se vuelve a leer sin reiniciar.
- **Seguridad**: las rutas de un descriptor se resuelven desde su carpeta y deben quedar dentro del paquete; la máscara es un SVG que pasa las mismas comprobaciones (§9.2); un descriptor mal formado, de otro esquema o versión, o con huecos fuera de la página, rechaza el paquete.

**Evidencia**

- `gafira-ui`: `s97_12_descriptor_paths_resolve_beside_it`, `s97_12_composite_descriptors_are_checked`, `s97_13_composite_assets_in_the_manifest` (escritas antes que su código y vistas fallar).
- `apps/gafira`: `s97_14_composite_assets_install` (marco con ranura, powerclip con `contain`, collage de dos huecos y plantilla instalados; un `.gaf` que no es plantilla se rechaza).
- Xephyr: `marcos.gafpack` (arco con foto, marco decorativo, PowerClip de corazón, collage de tres fotos y plantilla) se valida y se instala; la plantilla aparece en Plantillas al buscar el nombre del paquete y se abre como diseño nuevo; en Marcos, el arco y el corazón se insertan como marcos con foto; en Foto collage, el collage ocupa la página con sus tres huecos.

**Limitaciones**

- Un marco con foto usa solo la máscara: un adorno superpuesto al marco (por ejemplo un borde dibujado sobre la foto) aún no se admite en el mismo recurso; se puede ofrecer como marco decorativo aparte.
- Una plantilla de paquete abre un diseño nuevo; «Insertar en diseño actual» (§6.4) queda pendiente.

## 22. Rendimiento y herramienta de creación (2026-11-23)

**Instalación optimizada para paquetes grandes (§15.E)**

- El paquete nunca se lee entero en memoria: su índice se comprueba sin descomprimir (`by_index_raw`) y luego cada archivo se lee, se comprueba o se escribe por separado.
- La comprobación de los recursos se reparte entre unos pocos hilos (los núcleos menos uno, como mucho 4), cada uno con su propio lector del ZIP; la memoria queda en unos pocos archivos a la vez.
- La extracción copia cada entrada a disco en flujo (`io::copy`), sin cargarla en memoria.
- Inspección e instalación corren en un hilo propio: la ventana muestra una barra de progreso con porcentaje y megabytes, y la interfaz sigue respondiendo. La comprobación se puede cancelar; la instalación sigue hasta el final.
- Instalar un paquete recién inspeccionado y sin cambios (mismo tamaño y fecha) no lo vuelve a comprobar.
- El panel de recursos solo carga las vistas previas de las fichas visibles.
- Límites: 20 000 entradas, 4 GiB descomprimidos, 256 MiB por archivo.
- Evidencia: `s97_08_*` comprueba que el progreso de la instalación llega al total y que no hay segunda comprobación. En Xephyr, un paquete de 392 MB con 400 ilustraciones se comprobó con la barra visible y se instaló; la memoria máxima de toda la app (debug, render por software) fue de unos 290 MB y no subió al instalar.

**`scripts/gafpack.py`**

Crea un `.gafpack` válido desde una carpeta, para cualquier tipo de paquete (`clipart`, `illustrations`, `icons`, `shapes`, `frames`, `powerclips`, `collages`, `templates` o `mixed` con subcarpetas por tipo). Escribe el manifiesto 1.1, la miniatura `thumb.webp` (la indicada o un mosaico), una vista previa por imagen, los descriptores de marcos con foto, PowerClips y collages, y el hash SHA-256 de cada archivo. Aplica las mismas reglas que Gafira antes de escribir y comprueba el resultado (`check`). Los archivos se comprueban en paralelo y los originales se copian al ZIP desde el disco. Necesita Pillow con WebP.

- Evidencia: las 100 ilustraciones de Navidad (98 MB) se empaquetan en unos 2 s con 80 MB de memoria; un paquete mixto con los seis tipos se valida en Gafira.

