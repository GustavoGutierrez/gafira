# 104 — Creación en lote con datos conectados

> **Estado:** in-progress — 2026-11-26 — primera entrega implementada y probada (§19); pendientes las partes de §19.4.

**Especificación funcional y técnica propuesta · v1.2 · 26 de noviembre de 2026**

Aplicación objetivo: Gafira de escritorio, con arquitectura y procesamiento nativos en Rust. Público: personas habituadas a un editor 2D, sin conocimientos de programación ni de combinación de correspondencia.

Las secciones de investigación describen documentación pública consultada; las demás establecen decisiones propuestas para Gafira. No constituyen una auditoría práctica de cuentas de pago ni mediciones de rendimiento de los productos comparados. Las metas de velocidad deberán validarse con el motor real de Gafira.

## 1. Objetivo y modelo de uso

Convertir un diseño en una plantilla reutilizable y generar variaciones cambiando textos e imágenes mediante una tabla importada o escrita dentro de Gafira.

**Una columna representa un campo; una fila representa una variación; la plantilla decide qué elementos reciben cada campo.**

Ejemplos: certificados con nombres, carnés con fotografías, promociones con precios, invitaciones, tarjetas de productos y publicaciones para redes sociales.

Principios obligatorios:

- Conectar visualmente sin escribir variables; ofrecer escritura con autocompletado como alternativa rápida.
- Reutilizar los objetos de texto, estilos, efectos, marcos y renderizado existentes.
- Revisar datos y resultados antes de exportar, con correcciones desde el mismo contexto.
- Guardar tabla, conexiones y configuración dentro del `.gaf`; el archivo debe seguir funcionando sin la hoja original.
- Mantener la interfaz disponible mientras se importa, valida o genera.
- No imponer por diseño topes comerciales de 99 o 300 registros; establecer límites técnicos medidos y transparentes.

## 2. Investigación: cómo lo resuelven otras aplicaciones

### 2.1 Canva: creación en lote

Canva vincula columnas con elementos de una plantilla y genera una versión por fila. Admite tabla manual, CSV, XLSX, Canva Sheets y aplicaciones conectadas. Para Excel con varias hojas permite seleccionar hoja y rango. Documenta conexión visual mediante arrastre y emparejamiento automático, además de selección de campo desde la herramienta de conexión. Permite vista previa y páginas o diseños individuales. [C1]

No exige escribir `{nombre}` ni `{{nombre}}`: la conexión es una propiedad del elemento. La ayuda vigente habla de «Match data» y «Connect data»; no establece una sintaxis textual pública obligatoria. Por tanto, Gafira puede adoptar la interacción visual sin declarar compatibilidad de sintaxis con Canva. Gafira añadirá «Conectar datos» al menú contextual, sin presuponer idéntico acceso en todas las versiones de Canva. [C1]

La página general declara hasta 150 campos conectados y una selección de Canva Sheets de hasta 300 filas y 150 columnas. [C1]

**Discrepancia oficial:** otra página de Canva Sheets menciona 300 filas y 60 columnas para la aplicación Bulk Create, pero indica que Canva Sheets no tiene un límite específico de filas o columnas. También advierte que diseños complejos pueden dividirse en varios archivos. No existe, con estas fuentes, un único límite universal verificable para todas las rutas de Canva. [C2]

Canva Sheets no admite enlaces directos de imágenes en este flujo; utiliza medios cargados y solo el primer medio de cada celda. Su previsualización muestra videos como fotogramas y algunos elementos 3D pueden fallar. La sustitución de medios en la vista previa tiene restricciones según la fuente. [C2]

**Observación de uso aportada por el usuario:** en el flujo de Canva que ha utilizado, al generar se abre una nueva pestaña del navegador con otro diseño de Canva que contiene todas las páginas ya rellenadas. Esta observación concreta complementa las opciones documentadas: el resultado también puede ser un documento editable dentro del editor, no solamente archivos exportados. No se generaliza la apertura de pestaña a todas las rutas o versiones.

### 2.2 Photoshop: variables por capa y conjuntos de datos

El usuario define variables desde `Imagen > Variables > Definir`, asignándolas a capas. Hay sustitución de texto, sustitución de píxeles y visibilidad. Las imágenes permiten ajuste y alineación. Después se crean o importan conjuntos de datos y se revisan las variaciones. La exportación documentada genera un PSD separado por conjunto. El flujo está organizado por capas y diálogos, más técnico que un panel guiado. [P1]

La importación usa CSV o texto tabulado: primera fila con nombres de variables y siguientes filas con los valores. Para imágenes admite rutas; para visibilidad usa valores booleanos. No utiliza llaves escritas dentro del lienzo como mecanismo principal. La documentación consultada no ofrece selección directa de una hoja XLSX en este flujo. [P2]

### 2.3 Adobe Express: distinguir dos funciones

La función nativa «Bulk create» permite cargar CSV, seleccionar elementos, conectarlos con columnas y crear hasta **249 variaciones**. [E1]

El complemento distinto «Bulk Create and Generate» documenta hasta **99 variaciones**, una plantilla CSV generada desde el propio complemento, etiquetas de texto representadas como `{Text string}` e imágenes JPEG/PNG. No se debe atribuir su límite de 99 a toda la creación en lote de Express. [E2]

### 2.4 InDesign: campos dentro de texto y combinación editorial

InDesign inserta campos desde el panel Data Merge y los muestra como `<<nombre>>`. **Escribir manualmente esos caracteres no crea un campo válido.** El campo puede recibir formato de caracteres; las imágenes se conectan a marcos y sus columnas se identifican con `@` en el origen. [I1]

Permite selección de registros, varias fichas por página, avisos de imágenes faltantes y texto desbordado. La combinación directa a PDF reutiliza contenido estático con XObjects. La colocación de varios registros por página tiene restricciones en documentos de varias páginas y páginas padre. [I2]

La API avanzada de InDesign también documenta reglas y reducción de tamaño de texto dentro de límites configurados. Por tanto, el ajuste automático no debe presentarse como una innovación exclusiva de Gafira ni como una ausencia universal de los competidores. [I3]

### 2.5 Comparación de conexiones e identificadores

| Aplicación | Forma principal de vincular | Representación / identificador | Lección para Gafira |
| --- | --- | --- | --- |
| Canva | Conectar elemento con columna; arrastre y emparejamiento | Campo conectado, sin sintaxis de llaves exigida por la ayuda consultada | Priorizar conexiones visuales |
| Photoshop | Variable asignada a una capa | Nombre de variable en diálogos y cabecera CSV | Separar nombre visible e identidad interna |
| Express nativo | Conectar elemento y elegir columna | Vínculo visual | Reducir pasos repetitivos |
| Express complemento | Etiqueta vinculada y CSV generado | `{Text string}` documentado en ese complemento | Llaves simples son una convención particular |
| InDesign | Insertar desde panel Data Merge | `<<nombre>>`, con objeto de campo real | Guardar campos estructurados y conservar estilos |
| **Gafira propuesto** | Arrastre, menú contextual, botón o escritura | `{{nombre}}`, convertido en campo estructurado | Unificar facilidad visual y rapidez de teclado |

No se afirma que `{{nombre}}` sea el estándar de Canva: es la sintaxis elegida para Gafira por su visibilidad y menor colisión con texto corriente.

## 3. Alcance y entregas

### Primera versión utilizable

CSV, TSV, XLSX; selección de hoja y encabezados; tabla manual y pegado; campos de texto e imagen; texto mixto; autocompletado; conexión por arrastre y menú; previsualización por registro; validación; exportación masiva; persistencia completa en `.gaf`; cola cancelable con memoria limitada.

La primera versión genera una página o un conjunto ordenado de páginas de plantilla por fila. Exporta mediante los formatos estáticos que el motor de Gafira ya soporte: como mínimo PNG, JPEG y PDF si estos exportadores forman parte del producto. No crear un segundo motor solo para este módulo.

### Ampliaciones

- Visibilidad condicional, formateadores avanzados y regeneración selectiva.
- Colocación de varios registros en una hoja imprimible.
- Importación de imágenes embebidas en XLSX, después de validar sus variantes reales.
- Fuentes remotas actualizables, múltiples tablas relacionadas y procesamiento de video.

La disponibilidad de un formato en una dependencia no equivale a soporte aprobado de Gafira. XLS, ODS, macros, fórmulas recalculadas, relaciones entre tablas y edición masiva de modelos 3D quedan fuera de la primera entrega.

## 4. Acceso y organización de la interfaz

Nombre: **«Crear en lote» / «Bulk create»**. Accesos desde herramientas del editor, panel Datos y menú contextual de un elemento compatible.

Panel lateral con una secuencia persistente: **Datos → Conectar → Revisar y generar**. No bloquear el lienzo con un asistente modal durante todo el proceso. El usuario puede volver a cualquier paso sin perder trabajo.

Tras cargar datos se muestra: `Clientes · 1.240 filas · 8 campos`, botón «Editar tabla», búsqueda de campos y número de conexiones por columna. El documento lleva una insignia discreta «Datos conectados»; las capas conectadas muestran un icono independiente del nombre.

En pantallas estrechas la tabla se abre en un área ampliable; volver al lienzo conserva selección, filtros y posición de edición. Los ejemplos precargados deben identificarse como demostración y no mezclarse con los datos reales.

## 5. Importación de datos

### 5.1 Elegir origen

Tres acciones principales: «Subir CSV o Excel», «Pegar desde una hoja» y «Crear tabla». Arrastrar un archivo al panel Datos abre el mismo flujo.

### 5.2 XLSX: hoja, rango y encabezados

1. Leer metadatos y listar hojas por nombre; marcar las ocultas. Mostrar vista previa de la hoja seleccionada, sin cargar indiscriminadamente todas.
2. Si hay una única hoja con datos, preseleccionarla. Si hay varias, preseleccionar la primera con datos y hacer visible el selector antes de confirmar.
3. Proponer región utilizada y permitir rango como `A3:H240`. Si hay tablas nombradas y el importador puede extraerlas correctamente, ofrecerlas sin hacerlas requisito.
4. Proponer la fila de encabezados dentro del rango y resaltarla: «Usar fila 3 como encabezados». Permitir elegir otra fila o «No hay encabezados».
5. Mostrar nombres de campos, primeras filas, tipos propuestos y advertencias. Botón «Cargar 237 filas» con el recuento después de exclusiones.

Heurística: evaluar hasta 20 filas iniciales del rango, favoreciendo títulos no vacíos y diferenciados y consistencia de las filas siguientes. Es una sugerencia corregible; nunca descartar filas por una inferencia invisible.

| Caso | Comportamiento requerido |
| --- | --- |
| Encabezado vacío | Proponer «Columna A»; permitir renombrar |
| Nombres duplicados | Conservar nombre original y proponer alias únicos; mostrar la distinción |
| Encabezados combinados | Avisar y pedir fila alternativa o nombres editables; no adivinar jerarquías |
| Filas vacías | Proponer excluir filas totalmente vacías, mostrando cuántas |
| Filas ocultas o filtradas | Opción explícita de inclusión; si el lector no conserva esa información, avisarlo |
| Fórmula con resultado guardado | Importar el valor almacenado e indicar que no se recalcula |
| Fórmula sin resultado o con error | Marcar celda; permitir corregir o sustituir; no convertirla en cero |
| Libro protegido, cifrado o corrupto | Explicar el motivo y pedir copia legible; conservar la sesión actual |

La selección inicial importa una sola hoja. Cambiar hoja después se trata como reemplazar el origen con remapeo, no como anexar filas silenciosamente.

### 5.3 CSV, TSV y portapapeles

Detectar BOM, UTF-8 y separadores habituales: coma, punto y coma y tabulación. Ofrecer corrección de codificación y separador cuando la vista previa resulte ambigua. Soportar comillas escapadas, separadores y saltos de línea dentro de celdas. No usar división ingenua por comas o líneas.

Advertir sobre filas con diferente número de celdas y permitir inspeccionarlas. No perder columnas sobrantes ni completar silenciosamente valores inexistentes sin informar.

Pegar desde Excel o Sheets abre una vista previa tabular con opción «La primera fila contiene encabezados». Pegar en una celda del editor de tabla distribuye el bloque desde esa posición y permite deshacerlo como una sola operación.

## 6. Tabla interna y tipos

Tabla virtualizada: solo pintar filas y columnas visibles. Cabecera fija, navegación con teclado, edición por celda, pegar rangos, añadir filas/columnas, duplicar, buscar, filtrar, ordenar y deshacer. Las filas tienen identidad estable aunque se reordenen.

Tipos visibles: texto, número, fecha, booleano e imagen. El tipo inferido es modificable. Mantener valor original y valor tipado cuando difieran; importación y formato de salida son operaciones separadas.

- Códigos como `00125`, teléfonos y documentos se conservan como texto cuando el origen los proporciona así. Si Excel ya perdió los ceros, no se pueden reconstruir con certeza.
- Números y fechas se interpretan con configuración regional visible; ante `1.234` o `03/04/2026` ambiguos, pedir revisión del formato, no inventar el significado.
- Usar decimal exacto para precios; aplicar moneda y separadores en el campo conectado, sin alterar el valor fuente.
- Diferenciar celda vacía, cadena vacía, cero y falso. Un texto vacío intencional no se reemplaza automáticamente por el texto de ejemplo.
- Preservar saltos de línea y Unicode; advertir por caracteres sin glifos.

Eliminar una columna conectada presenta cuántas conexiones afecta y permite cancelar, reasignar o dejar campos pendientes. Renombrarla conserva conexiones por identificador interno.

### Actualizar la hoja original

Los datos son una copia editable, sin sincronización silenciosa. «Actualizar desde archivo» muestra cambios propuestos y conflictos con ediciones manuales. Remapear primero por identidad conocida y luego por nombres exactos únicos; coincidencias aproximadas requieren revisión. Para comparar filas se puede elegir una columna clave única; sin clave no se asume que el mismo número de fila representa la misma persona. Conservar la versión anterior hasta confirmar la actualización.

## 7. Conectar campos al diseño

### 7.1 Campos arrastrables

Cada columna aparece como ficha con icono de tipo, nombre, ejemplo de la fila actual y contador de uso.

| Destino | Resultado |
| --- | --- |
| Lienzo vacío | Crear texto normal con campo; para imagen, crear un marco |
| Caja de texto seleccionada como objeto | Proponer reemplazar su contenido con el campo, conservando el estilo base |
| Texto en modo edición | Insertar en el cursor o sustituir la selección |
| Marco o imagen compatible | Conectar la imagen y conservar máscara, transformación y efectos compatibles |
| Objeto incompatible | Resaltar incompatibilidad y explicar qué elemento usar |

Al reemplazar texto con varios estilos, mostrar qué se sustituirá; preservar el estilo en el punto de inserción y no prometer conservar rangos internos que dejarán de existir. Todas las conexiones son reversibles con deshacer.

### 7.2 Menú contextual y propiedades

`Clic derecho → Conectar datos → [Buscar columna…]`. Mostrar todas las columnas con búsqueda y compatibilidad; las incompatibles aparecen deshabilitadas con explicación. Sin datos, ofrecer «Cargar datos» y regresar al elemento al terminar.

En texto: «Reemplazar contenido» o «Insertar campo» según el modo de edición. En imagen: «Usar imagen de…». En propiedades debe existir el mismo acceso para usuarios de teclado. Una columna puede alimentar tantos elementos como sean necesarios.

Opciones de un campo conectado: cambiar columna, ajustar formato, definir valor de respaldo, localizar en tabla y desconectar. Desconectar permite conservar el valor actual como texto o restaurar el contenido original de esa conexión; no borra la columna.

### 7.3 Texto mixto, variables y autocompletado

Sintaxis propuesta: `{{nombre}}`. Ejemplos:

```text
Hola, {{nombre}}.
Has completado {{curso}} el {{fecha}}.
Precio: {{precio}} · Código: {{codigo}}
```

Reglas:

1. Con datos activos, escribir `{{` abre sugerencias; también se puede activar «Insertar campo» desde la barra de texto.
2. Filtrar por nombre y alias, ignorando mayúsculas y tildes solo para la búsqueda. Flechas navegan, Enter o Tab insertan y Escape cierra. Respetar composición de teclado y métodos de entrada.
3. Una sugerencia muestra nombre original, tipo, ejemplo y origen. No aceptar una coincidencia ambigua automáticamente.
4. Al completar una referencia válida, convertirla en un nodo de campo del texto enriquecido con `column_id`; las llaves son representación editable, no un reemplazo global ejecutado en cada imagen.
5. Un campo desconocido se subraya y ofrece conectar o crear columna. Texto entre llaves simples se conserva literalmente.
6. Para encabezados complejos, autocompletar `{{["Nombre completo"]}}`. El contenido entre corchetes usa una cadena JSON con escapes definidos. Las claves simples siguen `[A-Za-z_][A-Za-z0-9_]*`; la UI genera alias únicos, conservando los títulos Unicode originales.
7. `\{{` introduce llaves dobles literales y `\\` una barra literal en la representación de plantilla. El editor ofrece además «Tratar como texto». Las cadenas de los datos nunca se vuelven a interpretar como variables.
8. Pegar texto con variables muestra una propuesta de conversión cuando el objeto no estaba en modo de plantilla. No transformar automáticamente documentos corrientes que contengan llaves como contenido.
9. Copiar campos dentro de Gafira conserva su identidad; al pegarlos en otro documento, intentar remapear por nombre único y pedir revisión si no hay coincidencia.

Primera versión: sin expresiones ejecutables dentro de las llaves. Fechas, monedas, mayúsculas y respaldos se configuran visualmente y se almacenan como opciones estructuradas. No usar `eval`, JavaScript ni scripts de hoja de cálculo.

### 7.4 Reutilización del formato de texto

No crear un nuevo tipo de caja separado del editor actual. Insertar un nodo de campo entre los segmentos existentes. Su valor resuelto hereda tipografía, tamaño, peso, color, interletrado, interlineado, alineación, contorno, sombra y demás efectos compatibles que ya soporte Gafira.

Un campo tiene estilo uniforme por segmento y puede recibir formato independiente del texto fijo. Los valores de una celda son texto plano: no introducen HTML, formato oculto ni código. Resolver el campo vuelve a pasar por el mismo motor de composición tipográfica y ajuste de líneas del editor.

### 7.5 Conexión sugerida

«Sugerir conexiones» compara nombres de columnas con nombres de capas y marcadores reconocidos. Presenta una lista confirmable; no reemplaza textos por similitud débil. El flujo básico funciona localmente y no requiere IA ni conexión a Internet.

## 8. Imágenes y contenido condicional

Primera versión: imagen elegida en tabla, recurso ya importado o archivo local relativo a una carpeta seleccionada. «Buscar imágenes en carpeta» relaciona nombres de archivo y muestra coincidencias, faltantes y duplicados antes de aplicar. No buscar archivos por todo el equipo.

Opcional en una ampliación: URL HTTPS directa. Descargar una vez por recurso, aplicar límites de tamaño, tiempo y redirecciones, validar tipo real y almacenar una copia para portabilidad. Las URLs que son páginas de un servicio de fotos no equivalen a archivos directos. No guardar credenciales ni tokens en el `.gaf`.

Ajustes por conexión: encajar, cubrir, tamaño original, alineación y punto focal. Conservar relación de aspecto salvo deformación expresamente elegida. Si falta un archivo, usar respaldo configurado o marcar error, nunca reutilizar silenciosamente la foto de otra fila.

Las imágenes embebidas de Excel requieren un adaptador específico: una imagen flotante, una imagen en celda y una fórmula de imagen no tienen necesariamente la misma estructura. No asociarlas a la fila más cercana sin confirmar. Si no se soporta la variante detectada, indicarlo y ofrecer importar imágenes desde carpeta.

Ampliación: reglas visuales «Mostrar cuando…» aplicadas a un objeto o grupo, con operadores permitidos como «está vacío», «es igual a» y comparaciones numéricas. Configurar si ocultar conserva el espacio o activa un reajuste compatible; no prometer maquetación automática que el editor no tenga.

## 9. Revisar resultados y corregir problemas

Modo conmutador «Plantilla / Vista previa». Navegar primera, anterior, siguiente, última y buscar por columna; el contador distingue posición filtrada e identificador de fila. No modificar la plantilla al avanzar entre personas.

La vista previa renderiza la fila actual y precarga un pequeño vecindario cancelable. La tira de miniaturas se virtualiza y se genera bajo demanda, no para toda la tabla al importar.

### Texto que no cabe

Políticas por caja:

- **Avisar**, predeterminada: conservar composición y reportar desbordamiento.
- **Reducir hasta…**: reducir dentro de un mínimo definido; si aún no cabe, mantener la incidencia.
- **Ampliar caja**: solo sobre los ejes permitidos y mostrar posibles solapamientos.
- **Recortar con puntos suspensivos**: opción explícita para casos apropiados; nunca predeterminada para nombres o certificados.

La reducción usa búsqueda acotada y medición real del texto, incluyendo efectos y escritura compleja. No reducir todas las cajas por el registro más largo salvo que el usuario elija «Tamaño uniforme para todo el lote».

### Validación antes de generar

Validar conexiones, valores requeridos, imágenes, fuentes, glifos, dimensiones, desbordamientos y nombres de archivo. Ofrecer revisión rápida de primera/última fila y valores largos; identificarla como muestreo. «Validar todas las filas» procesa todo el lote con progreso y cancelación. La ejecución vuelve a validar cada salida, incluso después del muestreo.

Lista de incidencias con fila, campo, objeto y acción «Ir al problema». Corregir una celda y regresar a la misma vista. No usar solamente color: incluir icono y texto, accesibles con teclado y lector de pantalla según las capacidades del framework nativo.

Errores estructurales bloquean el inicio: columna ausente, plantilla inválida o destino sin escritura. Errores por fila permiten «Corregir», «Excluir estas filas» o «Generar las válidas». Las advertencias aceptadas quedan registradas; nunca omitir registros sin contabilizarlos.

## 10. Generación, páginas y exportación

### 10.1 Alcance de la plantilla

Seleccionar explícitamente las páginas de plantilla. Si son `P` páginas y hay `N` filas elegibles, el resultado es `N × P` páginas, ordenadas por fila y por orden de plantilla. Las páginas sin conexiones incluidas se repiten; mostrar este hecho en el resumen.

No confundir esta modalidad con colocar varios registros en una página. Esa ampliación define cuadrícula, márgenes, separación, orden de llenado, sangrado y manejo de la última hoja incompleta.

### 10.2 Salidas

| Opción | Uso y comportamiento |
| --- | --- |
| Exportar archivos | Generar directamente PNG/JPEG u otro formato estático soportado; no abrir páginas en el editor |
| PDF combinado | Todas las páginas en orden; escritura incremental o ensamblado por bloques según el exportador |
| PDF por registro | Un documento de P páginas por fila |
| **Generar páginas editables** | Crear las páginas con datos reemplazados y contenido editable. Su presentación, guardado y acceso se definirán durante el desarrollo según las capacidades existentes de Gafira, preservando la plantilla y los datos |
| Crear `.gaf` por registro | Copia editable resuelta, sin incluir por defecto la tabla completa de otras personas |

### Resultado editable y presentación por definir

El resultado funcional esperado es disponer de páginas con los datos ya reemplazados, conservando textos e imágenes editables. **Gafira actualmente no dispone de pestañas ni abre múltiples diseños simultáneamente. Esta función no debe exigir incorporar esas capacidades.** La referencia de Canva ilustra el resultado generado, no la estructura de navegación que deba copiar Gafira.

Durante el desarrollo se inspeccionará el modelo de documentos, páginas, navegación y guardado existente para decidir cómo presentar y acceder al resultado. No se fija aquí apertura automática, documento adicional abierto, cambio de documento, generación visible en segundo plano ni nuevas ventanas. La decisión debe documentarse antes de implementar el flujo y reutilizar las capacidades reales del editor.

Requisitos independientes de esa decisión:

1. Generar todas las páginas previstas en el orden anunciado y distinguir claramente la plantilla del resultado resuelto.
2. Mantener los textos como objetos de texto con sus estilos y las imágenes con sus marcos y propiedades editables; no convertir las páginas en capturas aplanadas.
3. Permitir acceder al resultado para revisarlo, corregirlo, guardarlo y exportarlo mediante las funciones que se habiliten de acuerdo con el editor existente.
4. Preservar plantilla, tabla y conexiones. Si el flujo elegido requiere cerrar o sustituir el documento activo, proteger sus cambios con el mecanismo de guardado existente y evitar pérdida de trabajo. No es necesario mantener origen y resultado abiertos a la vez.
5. Generar un resultado resuelto e independiente: las correcciones manuales no modifican las filas originales y actualizar la tabla no sobrescribe ese resultado. Para regenerar se utiliza la plantilla preservada. Un eventual modo vinculado requeriría resolución explícita de conflictos.
6. Conservar procedencia mínima por página: ID de trabajo, revisión de plantilla e ID de fila. No copiar por defecto la tabla completa al resultado ni convertir la procedencia en un requisito para editarlo.

**Todas las páginas disponibles no significa todas las páginas renderizadas en RAM.** Reutilizar carga diferida y miniaturas bajo demanda si el editor las soporta; definir la estrategia de materialización y guardado de acuerdo con su modelo real. No exigir edición del resultado mientras continúa la generación. Si se habilita esa posibilidad, proteger las ediciones frente al generador y guardar snapshots consistentes del resultado parcial.

Para lotes cuyo tamaño estimado exceda el presupuesto de páginas editables, mostrar el cálculo antes de iniciar y ofrecer exportación directa o división en archivos de resultado, según el soporte implementado. No cambiar silenciosamente de salida. El umbral depende de páginas, objetos y recursos, y debe fijarse mediante mediciones, no solo por número de filas.

### Selección y nombres de salida

Antes de generar: elegir todas las filas, rango, selección o filtros actuales; mostrar `1.240 registros · 2 páginas por registro · 2.480 páginas`. La selección queda congelada al iniciar. Ordenar la tabla cambia el orden de salida solo antes de esa congelación.

Nombres mediante constructor visual y variables: `certificado_{{codigo}}_{{nombre}}`. Para imágenes de plantillas multipágina se añade sufijo de página. El sistema sanea caracteres prohibidos, nombres reservados, longitud y diferencias entre sistemas de archivos. Detecta colisiones antes de escribir; añade índice estable por defecto, sin sobrescribir archivos existentes silenciosamente. Mostrar cinco ejemplos reales.

Guardar carpeta y ajustes como preferencias del proyecto, pero volver a verificar que el destino exista y sea escribible en el equipo actual. Estimar espacio como rango aproximado; actualizar tras las primeras salidas. ZIP es opcional y por bloques para lotes grandes, sin cargarlo entero en RAM.

### 10.3 Trabajo en curso

Mostrar fase, completados, fallidos, pendientes, velocidad reciente y tiempo restante aproximado. No mostrar estimación hasta tener muestras suficientes. Acciones «Pausar», «Continuar», «Cancelar» y «Ver resultados».

Pausar deja de programar nuevos registros; termina o guarda de manera segura el trabajo en curso. Cancelar conserva salidas completas y descarta archivos temporales incompletos. Presentar resumen y opción de reintentar fallidos. Una escritura a disco lenta o sin espacio debe frenar la cola, no acumular imágenes ilimitadas.

Editar la plantilla durante la generación afecta una nueva revisión; el trabajo activo usa la instantánea original. El usuario ve «Este lote usa la versión guardada al iniciarlo» y puede iniciar otro al terminar.

## 11. Persistencia en `.gaf`

### 11.1 Extensión versionada del formato

Esta propuesta añade un bloque opcional al esquema vigente de `.gaf`; no presupone que se haya verificado su implementación actual ni sustituye sus convenciones de recursos. Antes de programar, ajustar las rutas lógicas al contenedor real.

Información obligatoria:

- Versión del esquema de datos conectados y capacidades utilizadas.
- Tabla normalizada completa: columnas con ID estable, nombre original, alias, tipo y configuración regional; filas con ID estable, orden y valores.
- Origen informativo: tipo de archivo, nombre, hoja, rango, fila de encabezados y reglas de importación. Ruta local opcional y removible; no es requisito para reabrir.
- Campos dentro del texto, conexiones de imágenes, formatos, respaldos y reglas.
- Páginas de plantilla, revisión de datos y revisión de plantilla.
- Filtros, orden, selección, fila de previsualización y perfil de exportación.
- Recursos necesarios incorporados y deduplicados por contenido.
- Resumen de trabajos y checkpoints disponibles, sin afirmar que los archivos exportados externos viajan en el `.gaf`.

Usar una marca como `has_connected_data: true` derivada de las conexiones y una capacidad `bulk_data_v1`. No confiar solo en la extensión del archivo para detectar este modo.

### 11.2 Esquema lógico orientativo

```json
{
  "connected_data": {
    "schema_version": 1,
    "active_dataset_id": "ds_clientes",
    "datasets": [{
      "id": "ds_clientes",
      "revision": 4,
      "storage_ref": "data/ds_clientes.sqlite",
      "source": {
        "kind": "xlsx",
        "file_name": "clientes.xlsx",
        "sheet": "Afiliados",
        "range": "A3:D503",
        "header_row": 3
      },
      "columns": [
        {"id": "col_nombre", "name": "Nombre completo", "key": "nombre", "type": "text"}
      ]
    }],
    "template_page_ids": ["page_1"],
    "bindings": [{
      "id": "bind_1",
      "element_id": "text_8",
      "target": "text.inline",
      "field_node_id": "field_1",
      "dataset_id": "ds_clientes",
      "column_id": "col_nombre",
      "required": true,
      "empty_policy": "error"
    }],
    "export_profile": {
      "mode": "files",
      "format": "png",
      "file_name_template": "certificado_{{nombre}}",
      "collision_policy": "append_stable_index"
    }
  }
}
```

Es un ejemplo parcial: el documento real conserva los nodos de texto enriquecido y el almacén contiene todas las filas. `storage_ref` es una ruta lógica dentro del proyecto, no un vínculo al CSV externo.

### 11.3 Estrategia de almacenamiento y guardado

Propuesta: almacén tabular interno SQLite para evitar reescribir un JSON gigantesco tras cada celda. Si el contenedor `.gaf` es empaquetado, mantener la base de trabajo fuera del paquete mientras se edita y producir una copia consistente para el guardado. No escribir SQLite directamente dentro de un ZIP ni copiar únicamente el archivo principal de una base activa con WAL pendiente. Usar backup/snapshot consistente y escritura atómica del `.gaf`.

Empaquetar imágenes y recursos accesibles necesarios; incluir fuentes solo cuando su licencia y el subsistema de fuentes lo permitan. Si faltan, avisar antes de generar. El archivo original Excel completo es opcional y no se incorpora por defecto: basta la tabla normalizada seleccionada.

Reabrir debe reconstruir conexiones por IDs, sin reinterpretar llaves a partir del texto mostrado ni requerir el archivo fuente. Una versión antigua de Gafira que desconozca la capacidad debe abrir en modo protegido o advertir antes de guardar, evitando borrar datos silenciosamente.

Guardar plantillas para distribuir: opción «Quitar datos personales y conservar campos», con tabla vacía o datos de ejemplo expresamente elegidos. La tabla de producción queda incorporada por defecto al guardar el proyecto de trabajo; la interfaz debe hacerlo visible.

## 12. Arquitectura nativa Rust

### 12.1 Componentes

| Componente | Responsabilidad |
| --- | --- |
| Importador | Inspeccionar archivo, escoger hoja/rango, normalizar y producir diagnósticos |
| Almacén tabular | CRUD, IDs, revisiones, filtros, orden y lectura paginada |
| Conexiones | Vincular columnas con nodos de texto, imágenes y reglas |
| Planificador | Congelar entradas, enumerar salidas y controlar recursos |
| Resolvedor | Aplicar una fila sobre la plantilla compartida, sin mutarla |
| Renderizador | Reutilizar motor de composición y exportación de Gafira |
| Escritor | Guardar archivos completos, ordenar PDF y registrar resultados |
| Persistencia | Guardar datos, recursos y conexiones en el `.gaf` |

Comunicación con la UI mediante comandos y eventos tipados, con identificadores de trabajo y revisión. Ninguna importación extensa ni render pesado debe ejecutarse en el hilo de interfaz. Evitar eventos por cada celda: comunicar progreso agregado a una frecuencia limitada.

### 12.2 Dependencias candidatas verificadas

- `csv`: lectura/escritura CSV y configuración de separadores. [R1]
- `calamine`: lectura nativa de hojas Excel; dispone de soporte opcional para extraer imágenes, pero asociarlas semánticamente a registros requiere validación y lógica adicional. No asumir cálculo de fórmulas. [R2]
- `rayon`: paralelismo de CPU con pool configurable. [R3]
- `rusqlite`: acceso a SQLite y soporte de backup según características activadas. [R4]

Fijar versiones y licencias al integrar, siguiendo el árbol de dependencias de Gafira. Estas bibliotecas son propuestas, no dependencias confirmadas del código actual. Reutilizar ejecutor de I/O, decodificadores, fuentes y motor gráfico existentes.

### 12.3 Plan de ejecución

1. Crear instantánea inmutable de plantilla, datos seleccionados, recursos y perfil de exportación.
2. Compilar campos a referencias de columna y estilos una vez. Crear un mapa de dependencias entre columnas y objetos.
3. Leer filas en bloques acotados; resolver únicamente los elementos variables.
4. Medir texto y resolver imágenes compartiendo recursos seguros para lectura.
5. Renderizar/encodificar con concurrencia limitada por CPU, RAM y recursos gráficos.
6. Escribir a archivo temporal y renombrar al completar; para PDF, ensamblar en orden sin acumular todas las páginas.
7. Registrar resultado y checkpoint; liberar buffers antes de solicitar más trabajo.

CSV puede leerse secuencialmente y alimentar bloques; no dividir arbitrariamente por saltos de línea porque hay celdas multilínea. XLSX es un contenedor comprimido: abrir solo la hoja elegida reduce trabajo, pero no garantiza lectura incremental con memoria constante. Medir el lector y aplicar presupuesto antes de aceptar archivos enormes.

### 12.4 Paralelismo adaptable

No crear un hilo por fila. Usar un pool de CPU y colas acotadas entre lectura, composición, render y escritura. El escritor de un PDF combinado conserva el orden mediante un buffer también acotado; si una página anterior tarda, la cola aplica contrapresión.

Estimar concurrencia inicial mediante:

```text
trabajadores = max(1, min(limite_cpu, limite_ram, limite_render))
limite_ram = floor(presupuesto_para_trabajos / memoria_estimada_por_trabajo)
```

Si un solo trabajo supera el presupuesto, no usar la fórmula para forzar su ejecución: recurrir a bloques/teselas si el motor lo soporta o avisar antes de empezar. Descontar memoria compartida y cachés del presupuesto. Un buffer RGBA de 4096 × 4096 ocupa aproximadamente 64 MiB antes de efectos, copias y codificación.

Medir unas primeras filas representativas y adaptar trabajadores y tamaño de bloque, con límites mínimos/máximos y sin oscilaciones constantes. Para trabajos pequeños usar ejecución secuencial cuando el coste de planificación supere el beneficio.

La GPU o los contextos de fuentes pueden imponer afinidad de hilo o serialización. Paralelizar solo las etapas demostradas seguras; no asumir que disponer de Rust hace compartible cualquier renderizador. Evitar multiplicar hilos si el codificador ya tiene paralelismo interno.

Perfiles visibles: «Equilibrado» predeterminado, «Menor consumo» y «Máxima velocidad». Traducen presupuestos y prioridades; los controles detallados de hilos y memoria permanecen en ajustes avanzados.

### 12.5 Cachés y reutilización correcta

- Compartir recursos inmutables con conteo de referencias y cachés LRU con presupuesto.
- Deduplicar lectura/decodificación de imágenes repetidas y caché de composición de textos idénticos con igual estilo.
- Cachear subárboles estáticos cuando su resultado no dependa de capas variables, máscaras, mezclas o filtros externos. No aplanar toda la plantilla si rompe esas relaciones.
- Reutilizar recursos estáticos del PDF cuando el exportador permita XObjects; conservar texto/vector cuando corresponda, no rasterizar siempre.
- Claves de caché incluyen revisión de plantilla, datos efectivos, fuentes, escala, color y ajustes de exportación.
- Invalidar solo dependencias afectadas; no recalcular todo el lote al editar una celda.

Rust permite control de memoria y concurrencia, pero la ventaja frente a otros programas deberá demostrarse. Resolución, efectos, fuentes, disco y decodificación pueden dominar el tiempo total.

### 12.6 Recuperación y determinismo

Guardar manifiesto del trabajo con hashes de entradas, versiones relevantes del motor, filas, orden, salidas y estados. El estado de interfaz puede persistirse en `.gaf`; el diario de ejecución se actualiza en almacenamiento local sin reempaquetar continuamente el proyecto. Se incorpora un checkpoint consistente al guardar.

Reanudar solo si coinciden plantilla, datos, recursos, perfil y entorno relevante. Verificar archivos ya terminados por tamaño/hash antes de omitirlos. Si cambian entradas, crear otro trabajo. No sobrescribir resultados nuevos con una reanudación antigua.

La paralelización no altera nombres ni orden. Equivalencia de imagen se valida con tolerancias acordadas cuando el backend GPU no garantice igualdad binaria entre equipos.

## 13. Mejoras frente a limitaciones observadas

| Limitación o fricción documentada | Respuesta propuesta | Condición de validación |
| --- | --- | --- |
| Topes de las rutas de Canva y Express [C1, C2, E1, E2] | Procesamiento por bloques sin esos topes comerciales | Publicar tamaños probados por hardware y complejidad |
| Restricción de enlaces de imagen de Canva Sheets [C2] | Archivos locales y, después, HTTPS descargado y embebido | Validación de recursos y reapertura sin red |
| Variables por diálogos y capas en Photoshop [P1] | Panel continuo, arrastre y menú contextual | Pruebas de uso con personas de editores 2D |
| Campos de InDesign que no se crean al teclear [I1] | Convertir sintaxis/autocompletado a nodos reales | Igual comportamiento por cualquiera de las entradas |
| Incidencias de texto e imágenes en combinación [I2] | Corrección contextual y política explícita de ajuste | Ningún fallo u omisión silenciosos |
| Restricciones de varios registros por página [I2] | Módulo de imposición separado de la plantilla multipágina | Casos de anverso/reverso y última hoja parcial |
| Restricciones de vista previa de medios/3D de Canva [C2] | Un mismo resolvedor para vista previa y exportación estática | Comparación visual; video fuera del alcance inicial |

Estas son oportunidades de producto, no afirmaciones de que todos los competidores carezcan de cachés, paralelismo o persistencia. Su arquitectura interna no se deduce de sus límites públicos.

## 14. Rendimiento: objetivos y medición

**Objetivos propuestos, no resultados obtenidos.** Equipo mínimo de referencia: CPU de 4 núcleos, 8 GB RAM y SSD; equipo ampliado: 8 o más núcleos, 32 GB RAM. Medir Windows, Linux y macOS con sus backends disponibles.

| Métrica | Objetivo inicial |
| --- | --- |
| Respuesta al escribir y navegar tabla | p95 ≤ 100 ms, excluyendo operaciones explícitas de importación |
| Autocompletado sobre 500 columnas | p95 ≤ 100 ms con datos ya cargados |
| Cambio de registro, plantilla estática simple y caché caliente | p95 ≤ 200 ms a escala de vista previa |
| Mostrar progreso | ≤ 1 s desde el inicio de una operación larga |
| Cancelación | Acuse visual ≤ 100 ms; dejar de programar inmediatamente y finalizar la unidad segura en curso |
| Memoria de generación | Acotada por presupuesto y colas; no proporcional al número de imágenes de salida |
| Integridad del lote | Todo registro seleccionado termina como completo, fallido o excluido explícitamente |

Corpus: 100, 1.000, 10.000 y 100.000 registros; 5, 50 y 500 columnas. **100.000 filas y 500 columnas son pruebas de estrés separadas; su combinación no se promete como soporte certificado inicial.** Probar primero CSV y tabla; medir XLSX por separado por su coste de lectura.

Escenas: texto simple a 1080 × 1080; texto mixto y varias fuentes; imágenes compartidas; imágenes únicas de alta resolución; efectos y transparencias; PDF multipágina. Incluir nombres largos, acentos, escrituras complejas, celdas vacías y errores.

Registrar tiempo de importación, tiempo al primer resultado, registros/s, RAM máxima, uso de CPU, memoria gráfica cuando esté disponible, latencia de UI y tiempo de guardado/reapertura. Separar caché fría/caliente y recursos locales/remotos. Comparar ejecución secuencial y paralela con idénticas entradas; mantener paralelismo solo donde mejore la medición. No fijar una promesa universal de diseños por segundo.

## 15. Seguridad de archivos e integridad

No ejecutar macros, fórmulas, scripts ni contenido de celdas. Límites explícitos de bytes descomprimidos, celdas y dimensiones de imágenes para archivos dañados o desproporcionados; calcularlos desde presupuestos de recursos y comunicarlos antes de perder trabajo.

Validar rutas del paquete `.gaf` y evitar extracción fuera del proyecto. Resolver imágenes externas solo desde ubicaciones elegidas. No usar valores de celdas como rutas de salida sin saneamiento. En exportación de tablas CSV ofrecer modo seguro para impedir interpretación accidental de fórmulas en otra hoja de cálculo, conservando intactos los datos internos.

No cargar datos en servicios externos para el flujo básico. Al compartir el proyecto, mostrar que incluye la tabla incorporada y ofrecer quitarla; nunca incluir esa tabla completa en un `.gaf` individual de un destinatario salvo elección expresa.

## 16. Criterios de aceptación

1. Un XLSX con tres hojas y encabezados en la fila 4 permite elegir hoja y fila; importa solo los registros del rango confirmado.
2. Importar CSV con punto y coma, comillas, ñ y celdas multilínea conserva las celdas correctas; las filas defectuosas se señalan.
3. Crear tabla manual y pegar un bloque produce los mismos campos conectables que importar un archivo.
4. Arrastrar, usar el menú o escribir con autocompletado crea conexiones equivalentes con IDs estables.
5. `Hola, {{nombre}}` conserva el estilo del texto fijo y el del campo; avanzar de fila no modifica la plantilla.
6. Renombrar una columna mantiene vínculos; eliminarla o reemplazarla genera un diagnóstico antes de dañar conexiones.
7. Códigos textuales con ceros iniciales, cero, falso y vacíos conservan su semántica; fechas ambiguas requieren configuración visible.
8. Guardar `.gaf`, moverlo a otro equipo y borrar la hoja original conserva datos, conexiones y recursos incorporados. Recursos no incorporables se reportan.
9. Con 1.000 filas y dos páginas seleccionadas se generan 2.000 páginas en el orden anunciado, sin importar el número de trabajadores.
10. Colisiones de nombres y archivos existentes no producen sobrescritura silenciosa.
11. Imagen faltante y texto que no cabe se localizan por fila y objeto; excluirlos cambia el contador visible.
12. Exportar un lote grande no materializa todas sus páginas en el editor ni acumula todos sus buffers en RAM.
13. Pausar/cancelar preserva archivos completos; reintentar fallidos no duplica los ya verificados.
14. Cambiar plantilla durante un lote no altera sus resultados; la siguiente ejecución usa la revisión nueva.
15. Guardar tras un cierre y reanudar exige entradas coincidentes y verifica salidas existentes.
16. La generación editable produce páginas con textos e imágenes ya reemplazados, conserva la plantilla y permite guardar y recuperar el resultado sin la hoja original. Su acceso y presentación se resuelven durante el desarrollo con las capacidades existentes; no se requieren pestañas ni múltiples diseños abiertos.
17. Generar un `.gaf` individual no expone filas de otras personas; publicar plantilla sin datos elimina valores y recursos asociados que ya no se utilicen.

## 17. Plan de implementación y decisiones previas

**Paso previo:** inspeccionar esquema `.gaf`, modelo de documentos y páginas, navegación y guardado, texto enriquecido, exportadores, seguridad de hilo del motor gráfico, recursos y mecanismos de deshacer actuales. Decidir y documentar cómo acceder a los resultados editables sin presuponer pestañas ni apertura simultánea de diseños. Adaptar esta especificación sin duplicarlos.

**Entrega A — Datos y conexiones:** importación, hoja/encabezados, tabla, IDs, arrastre, menú, campos en texto y guardado portable.

**Entrega B — Generación fiable:** vista previa, validaciones, exportación secuencial por cola acotada, resultados por fila, pausa/cancelación y guardado atómico.

**Entrega C — Escala:** mediciones, paralelismo adaptable, cachés, checkpoints, reintentos y pruebas de estrés. La entrega inicial ya debe evitar bloquear la UI y cargar todas las salidas; esta fase optimiza sobre una base correcta.

**Entrega D — Ampliaciones:** múltiples registros por hoja, reglas visuales, imágenes XLSX, recursos HTTPS y actualización avanzada de fuentes.

Quedan por cerrar con el código real: compatibilidad exacta de fuentes/efectos, estructura física del contenedor, exportación incremental de PDF, presupuestos por plataforma y variantes XLSX de imagen. No deben resolverse prometiendo soporte que el motor aún no tenga.

## 18. Fuentes primarias consultadas

Consulta: **26 de noviembre de 2026**. Se enlazan documentos públicos; sus interfaces y límites pueden cambiar. Las discrepancias se han señalado en lugar de elegir una cifra arbitraria.

- **[C1] Canva — Create designs in bulk.** [Consultar fuente](https://www.canva.com/help/bulk-create/)
- **[C2] Canva — Create designs in bulk with Canva Sheets.** [Consultar fuente](https://www.canva.com/en_au/help/bulk-create-from-sheets/)
- **[P1] Adobe Photoshop — Create data-driven graphics.** [Consultar fuente](https://helpx.adobe.com/photoshop/desktop/automate-tasks/data-driven-graphics/create-data-driven-graphics.html)
- **[P2] Adobe Photoshop — Import variable data sets from external files.** [Consultar fuente](https://helpx.adobe.com/photoshop/desktop/automate-tasks/data-driven-graphics/import-variable-data-sets-from-external-files.html)
- **[E1] Adobe Express — Bulk create.** [Consultar fuente](https://helpx.adobe.com/express/web/bulk-create-and-automate/bulk-create.html)
- **[E2] Adobe Express — Bulk Create and Generate add-on.** [Consultar fuente](https://helpx.adobe.com/es/express/web/bulk-create-and-automate/bulk-create-generate.html)
- **[I1] Adobe InDesign — Add and edit data fields.** [Consultar fuente](https://helpx.adobe.com/indesign/desktop/automation-and-scripting/merge-data/add-and-edit-data-fields.html)
- **[I2] Adobe InDesign — Merge records.** [Consultar fuente](https://helpx.adobe.com/indesign/desktop/automation-and-scripting/merge-data/merge-records.html)
- **[I3] Adobe Developer — Advanced Data Merge API.** [Consultar fuente](https://developer.adobe.com/firefly-services/docs/indesign-apis/guides/working-with-advanced-data-merge-api/)
- **[R1] Rust csv — Documentación de la biblioteca.** [Consultar fuente](https://docs.rs/csv/latest/csv/)
- **[R2] Rust calamine — Documentación de la biblioteca.** [Consultar fuente](https://docs.rs/calamine/latest/calamine/)
- **[R3] Rust Rayon — Documentación de la biblioteca.** [Consultar fuente](https://docs.rs/rayon/latest/rayon/)
- **[R4] Rust rusqlite — Documentación de la biblioteca.** [Consultar fuente](https://docs.rs/rusqlite/latest/rusqlite/)

## 19. Estado de la implementación (primera entrega, 2026-11-26)

### 19.1 Decisiones tomadas con el código real

| Tema | Decisión |
| --- | --- |
| Acceso | Pestaña «Crear en lote» del riel, entre Dibujo y Marca (icono `Stack`), con el panel de tres pasos **Datos → Conectar → Generar**. No hay menú contextual general de elementos en el lienzo; las acciones «Reemplazar» y «Añadir al final» del panel actúan sobre el texto seleccionado. |
| Campos en el texto | Los campos se guardan como texto `{{clave}}` dentro de los tramos (`Span`) del texto enriquecido, no como un nodo nuevo: el editor de texto en el lienzo trabaja con texto plano y un nodo propio exigiría reescribirlo. La identidad estable es la **clave** de la columna: renombrar la columna cambia su nombre visible y conserva la clave, así que las conexiones siguen funcionando. `{{["Título"]}}` nombra por título original. |
| Tabla en el `.gaf` | Bloque opcional `connected_data` en `document.json` (`DataSet`, versión 1), escrito solo cuando hay tabla; los documentos sin datos conservan sus bytes. No se usa SQLite: la tabla viaja entera y las filas se comparten por `Arc` entre pasos de deshacer. |
| Resultado editable (§10.2) | Gafira no abre varios diseños a la vez: el resultado editable es **un archivo `.gaf` nuevo**, con una página por fila y página de plantilla, textos reemplazados y sin la tabla de datos. La plantilla abierta no cambia y no se cierra. |
| Exportación | Reutiliza los exportadores PNG, JPEG y PDF del motor sobre un documento resuelto aparte (`ExportService::prepare_document` y `PreparedExport::again`), con las opciones predeterminadas de cada formato. Los archivos por fila se resuelven fila a fila, sin tener todas las páginas en memoria; el PDF combinado y el `.gaf` necesitan el documento completo. |
| Cuadrícula de datos | La cuadrícula del editor de datos de los gráficos se extrajo a `data_grid` y la usan los gráficos y la tabla del lote (panel y ventana «Editar tabla»). |
| Texto que no cabe | Política «Avisar»: las cajas que crecen (ancho o alto automáticos) siguen a sus palabras como al escribir; las de tamaño fijo que no alcanzan se informan por fila y elemento al terminar. |

### 19.2 Hecho

- Importar CSV/TSV/XLSX/XLSM en segundo plano, con elección de hoja (diálogo de la spec 082), fila de nombres sugerida y corregible, filas vacías omitidas y filas desiguales avisadas; pegar desde una hoja; crear una tabla y escribirla en la cuadrícula (añade filas y campos al escribir fuera).
- Claves únicas ASCII a partir de los títulos (`Año de ingreso` → `ano_de_ingreso`), títulos sin nombre «Columna A».
- Conectar: fichas por campo con ejemplo de la fila y número de usos; arrastrar al lienzo, «Añadir texto», «Reemplazar», «Añadir al final», «Copiar»; autocompletado al escribir `{{` en el lienzo (↑ ↓ e Intro). Campo especial `{{fila}}`.
- Revisar: vista previa renderizada de cada fila y página de plantilla, navegación por filas, campos sin columna (bloquean, con «Ir»), filas con campos vacíos (aviso).
- Generar: páginas de plantilla, todas las filas o un rango, PDF combinado, PDF por fila, PNG, JPEG o solo `.gaf`, y la opción de `.gaf` editable además; nombres de archivo con campos, saneados y sin sobrescribir (`-2`, `-3`); progreso con fase, hechos/total, fallidos, faltan, archivo en curso, tiempo y estimación, botón «Cancelar», tarjeta flotante con el panel cerrado e informe final.

### 19.3 Evidencia

- Pruebas nuevas (`git grep s104_`): `gafira-doc` `tests/data.rs` (6), `gafira-edit` `tests/data_commands.rs` (2), `gafira-bulk` `tests/template.rs` (8) y `tests/resolve.rs` (7), `gafira-io-xlsx` `s104_30`, `gafira-engine` `s104_40`, `gafira-ui` `bulk::tests` (S104-50 a 55 y 57) y `editor::tests` (S104-56 y 58). S104-50 y 51 escriben PNG, PDF y `.gaf` reales y reabren el `.gaf`.
- Pruebas de propiedades: `set_data_obeys_the_command_laws` (V14) en `gafira-edit/tests/prop_commands.rs`.
- Gates (2026-11-26): `cargo fmt --check` OK; `cargo clippy --workspace --all-targets --locked -- -D warnings` OK; `cargo test --workspace --locked --no-fail-fast` 3805 pasadas, 0 fallidas en 301 binarios; pasos de política de `ci.yml` V5, V12, ARQ-107 (dos), V13 y V14 OK en local; `python3 tools/sync_agents.py --check` OK. Sin dependencias nuevas de terceros (`gafira-bulk` usa `serde_json` del workspace).
- No verificado: uso manual en la app de escritorio (diálogos del sistema y arrastre reales); las pruebas dibujan frames completos del editor y escriben los archivos, pero no abren ventanas.

### 19.4 Pendiente

- Imágenes conectadas a columnas (§8) y reglas de visibilidad.
- Políticas «Reducir hasta», «Ampliar caja» y «Recortar con puntos suspensivos» (§9).
- Formatos de número, fecha y moneda por campo, valores de respaldo, tipos de columna visibles y configuración regional (§6).
- Buscar, filtrar, ordenar y deshacer por celda en la tabla; «Actualizar desde archivo» con remapeo (§6).
- Elegir rango con letras (`A3:H240`) y tablas nombradas de XLSX (§5.2).
- Pausar, reintentar fallidos, reanudar tras cerrar y manifiesto del trabajo (§10.3, §12.6); perfiles de rendimiento y mediciones (§14).
- «Quitar datos personales y conservar campos» al distribuir plantillas (§11.3) y `.gaf` por registro.
- Tab en el autocompletado: el foco de egui lo toma antes que el campo; hoy se inserta con Intro.

### 19.5 Ajustes tras la primera prueba del titular (2026-11-26)

- Etiqueta del riel en dos líneas cuando no cabe («Crear» / «en lote»).
- Botones del panel con aspecto de botón (principal en coral, secundarios con contorno) y explicación al pasar el ratón por «Añadir texto», «Reemplazar» y «Añadir al final».
- El panel ya no se ensancha solo: la fila de pasos y las fichas cabían con unos píxeles de más (S104-61 lo comprueba fotograma a fotograma).
- «Crear tabla» abre un borrador con «Usar esta tabla» y «Cancelar» (S104-60).
- Soltar un CSV o Excel sobre el panel carga sus datos, con un aviso mientras se arrastra (S104-62).
- Clic derecho sobre un texto o una forma: «Conectar datos (reemplazar el texto)» y «Añadir un campo al final» con las columnas, o «Conectar datos…» que abre el panel cuando no hay datos (§7.2, con la spec 105).

### 19.6 Rediseño de la exportación (crítica de diseño, 2026-11-26)

Tras una crítica con la skill `impeccable` (20/40 en las heurísticas de Nielsen: inconsistencia con «Exportar» y demasiadas decisiones en 320 px), el titular aprobó:

- El paso 3 del panel pasa a «Revisar»: vista previa por fila, problemas con «Ir» y un único botón «Exportar en lote…».
- Diálogo «Exportar en lote» con la misma estructura que «Exportar»: vista previa de la fila a la izquierda; a la derecha el formato y sus opciones (todos los formatos de «Exportar» salvo Excel, más «Gafira editable (.gaf)», con la configuración recordada por formato), «Un solo archivo / Un archivo por fila» para los formatos de varias páginas, filas, páginas de la plantilla, nombres de archivo con fichas y ejemplos, y «También un .gaf editable»; abajo el resumen y «Exportar N páginas».
- El progreso y el resultado son una tarjeta abajo a la derecha, como la de «Exportar»: fase, hechos de total, fallidos, faltan, archivo en curso, tiempo y «Cancelar»; al terminar, «Mostrar en carpeta», «Ver detalles» (fallos y textos que no caben) y «Cerrar».
- `BulkProfile` guarda el id del formato (`pdf`, `png`… o `gaf`) y `one_file`, en lugar de la lista fija anterior.
- Evidencia: S104-55 (diálogo: formatos, filas, perfil, solo `.gaf`) y S104-63 (el diálogo se dibuja y no escribe nada mientras está abierto).

