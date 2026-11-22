# Gafira — Catálogo de fotos remotas

**Versión:** 1.0  
**Fecha:** 21 de noviembre de 2026  
**Estado:** in-progress — 2026-11-21 — D-111. Fase 0 (sin clave): Openverse y Wikimedia Commons; Pixabay, Unsplash y Pexels pendientes de intermediario o clave.  
**Ubicación:** Elementos → Fotos

## 1. Objetivo

Permitir que las personas busquen fotografías de Pixabay, Unsplash y Pexels dentro de Gafira, exploren sus resultados y las incorporen al lienzo o a un marco sin abandonar el editor.

La experiencia debe ser sencilla: abrir Fotos, escribir una búsqueda, aplicar filtros y seleccionar una imagen. El catálogo se consulta de forma remota; solo se incorporan al proyecto los recursos seleccionados, con el tratamiento permitido por cada proveedor.

Esta especificación describe comportamiento y contratos de integración. Los nombres técnicos son propuestos y deben ajustarse a la arquitectura real de Gafira sin introducir un segundo sistema de recursos o historial.

## 2. Alcance y prioridades

> **Fase 0 (D-111, en curso):** antes de las fases siguientes, Openverse y Wikimedia Commons sin clave de API, con licencias de uso comercial y modificación (CC0, dominio público, CC BY, CC BY-SA). Búsqueda con espera de 400 ms, accesos temáticos, orientación donde la fuente la admite (Openverse), miniaturas en segundo plano, clic y arrastre al lienzo o a un marco, autor y licencia visibles y guardados en el nombre del recurso. El adaptador `gafira-photos` y el puerto `PhotoCatalog` de `gafira-ports` sustituyen al intermediario mientras las fuentes no necesiten clave.
>
> **Hecho en la fase 0** (2026-11-21):
> - Puerto `gafira_ports::photos` (`PhotoCatalog`, `PhotoQuery`, `Photo`, `PhotoLicense` con sus elementos `LicenseTerm`, `PhotoRights`, `PhotoError`) y adaptador `gafira-photos` (Openverse, Wikimedia Commons, caché de la sesión, descanso tras un 429 según `Retry-After` o 60 s, un 5xx como «la fuente no responde», solo HTTPS, 5 MB por miniatura y 30 MB por imagen). Transporte `ureq` tras la feature `online`, solo en el binario. Pruebas con respuestas simuladas y una prueba real ignorada (`live_search`).
> - Derechos (titular): por defecto **libres de derechos** (CC0 y dominio público: cualquier uso, sin atribución); «Uso comercial» añade CC BY y CC BY-SA; «Todas las licencias» también NC y ND. Openverse filtra en su catálogo (`license=cc0,pdm`, `license_type=commercial,modification`); Wikimedia no tiene ese filtro y se filtra cada página, que puede llegar más corta.
> - Marcas (titular): cada foto muestra los elementos de su licencia (CC0, PD, BY, NC, SA, ND) con su explicación al pasar el puntero, y una marca verde «Libre» (libre de derechos) o azul «Uso comercial».
> - Panel: búsqueda con espera de 400 ms o Enter, fuente, derechos, orientación (solo Openverse), accesos temáticos, cuadrícula de dos columnas con miniaturas en segundo plano (6 a la vez), «Cargar más», autor y licencia bajo cada foto; un clic la inserta, un arrastre la suelta en la página o en un marco (D-105), un clic derecho abre su página. Una respuesta atrasada no sustituye a la de una búsqueda posterior. Al insertar se comprueba que sea una imagen de como mucho 80 Mpx, se guarda en el almacén con su crédito como nombre y se coloca en un paso de deshacer.
> - Wikimedia (2026-11-21, fallo del titular: arrastrar desde Wikimedia fallaba): `upload.wikimedia.org` responde 429 a los clientes HTTP/1.1 como `ureq`, y `thumb.wikimedia.org` no. La foto insertada es la miniatura de `thumb.wikimedia.org` al mayor ancho que Commons guarda (1920, 1280, 960, 500 o 330 px) sin pasar del original. Un 429 al bajar una imagen o una miniatura ya no deja en pausa la fuente; solo uno en la búsqueda (`a_429_to_an_image_does_not_rest_the_source`).
> - Pendiente: favoritos y «Usadas en este proyecto», vista previa grande, aviso de resolución para impresión, esqueletos de carga, cancelar una descarga, y las fuentes con clave.

| Fase | Entrega | Condición de activación |
| --- | --- | --- |
| 1 | Pixabay: búsqueda, filtros, vista previa, inserción y recursos persistentes | Clave válida y revisión del flujo de integración frente a sus términos |
| 2 | Unsplash: catálogo, créditos, eventos de descarga y renderizado remoto | Aprobación de producción y resolución explícita del flujo de exportación y guardado sin conexión |
| 3 | Pexels: búsqueda, filtros, colecciones e inserción | Clave existente válida o reapertura de nuevas claves; aprobación aplicable al caso de uso |

El lanzamiento inicial incluye fotografías estáticas. Videos, ilustraciones, vectores, eliminación de fondos y funciones generativas quedan fuera de esta entrega. Pixabay se consultará con `image_type=photo`.

No se descargará ni indexará una copia completa de los catálogos. Gafira no ofrecerá una biblioteca independiente de originales para redistribución.

## 3. Condiciones verificadas y dependencias externas

Los datos siguientes reflejan la investigación del 21 de noviembre de 2026. Deben revalidarse antes del lanzamiento y mantenerse configurables.

| Proveedor | Cuota publicada | Requisitos relevantes |
| --- | --- | --- |
| Pixabay | 100 solicitudes por 60 segundos por clave | Caché de respuestas de 24 horas; resultados remotos temporales; descargar imágenes antes de su uso persistente; sin descargas masivas |
| Unsplash | 50 solicitudes/hora en demo; 1.000/hora tras aprobación de producción | Mostrar mediante URL devuelta por API, créditos y enlace al fotógrafo, registrar la selección como descarga; proteger claves |
| Pexels | 200 solicitudes/hora y 20.000/mes | Enlace visible a Pexels y crédito al fotógrafo cuando sea posible; emisión de nuevas claves y ampliaciones de cuota actualmente pausadas |

Las licencias gratuitas mantienen condiciones. La interfaz utilizará «Fotos gratuitas» o «Fotos de [proveedor]», evitando prometer ausencia de derechos de autor o libertad irrestricta. Las personas, marcas y propiedades representadas pueden implicar derechos adicionales.

El uso como insumo de un diseño no equivale a autorización para redistribuir originales o venderlos sin transformación. La aplicación conservará procedencia y acceso a la licencia, sin insertar automáticamente marcas de agua ni créditos dentro del diseño exportado, salvo obligación concreta del proveedor o acuerdo aprobado.

## 4. Experiencia del panel

### 4.1 Estructura

1. Campo «Buscar fotos para tu diseño», con botón de búsqueda y limpieza.
2. Selector de fuente: Pixabay, Unsplash y Pexels. Solo se ofrecen fuentes habilitadas.
3. Filtros compactos y opción «Limpiar filtros».
4. Accesos temáticos: Naturaleza, Personas, Negocios, Comida, Tecnología, Fondos y Texturas.
5. Cuadrícula de resultados con carga progresiva.
6. Accesos a Favoritos y Usadas en este proyecto.
7. Identificación visible del proveedor y enlace correspondiente.

Las categorías temáticas de Gafira se traducen a consultas por proveedor; no se presentarán como taxonomías oficiales cuando no lo sean. El panel debe adaptarse al ancho disponible y respetar los temas claro y oscuro del editor.

### 4.2 Búsqueda

- Iniciar la búsqueda después de 400 ms sin escribir o inmediatamente al pulsar Enter.
- No enviar consultas de texto vacías: mostrar contenido inicial curado, popular o una selección temática según capacidades de la fuente.
- Conservar la consulta al cambiar de proveedor y reiniciar la paginación.
- Cancelar solicitudes anteriores; una respuesta tardía nunca reemplaza resultados de una consulta posterior.
- Guardar el último estado del panel durante la sesión del editor.
- Usar idioma español donde la API lo soporte. Unsplash documenta `lang` como parámetro beta: no depender de él sin acceso autorizado.
- No traducir consultas mediante un servicio adicional sin que esa integración esté configurada.

### 4.3 Filtros por capacidad

| Filtro | Pixabay | Unsplash | Pexels |
| --- | --- | --- | --- |
| Orientación | Horizontal, vertical | Horizontal, vertical, aproximadamente cuadrada | Horizontal, vertical, cuadrada |
| Color | Paleta predefinida | Paleta predefinida | Paleta y hexadecimal |
| Tamaño mínimo | Ancho y alto mínimos | Sin filtro equivalente en búsqueda documentada | Pequeño, mediano, grande según definición de API |
| Orden | Popular, reciente | Relevancia, reciente | Sin selector de orden equivalente en búsqueda documentada |
| Categorías | Categorías API | Colecciones; accesos temáticos propios | Colecciones; accesos temáticos propios |
| Contenido seguro | `safesearch` | `content_filter` | Sin parámetro equivalente documentado en búsqueda de fotos |

Los controles no compatibles se ocultan. Al cambiar de fuente se conservan únicamente los filtros equivalentes y se informa si alguno se ha retirado. No se simularán filtros de catálogo aplicándolos solo a una página de resultados sin explicarlo.

### 4.4 Tarjetas y vista previa

Cada tarjeta mostrará miniatura, autor y fuente. Sus acciones serán «Agregar al lienzo», «Vista previa» y «Guardar en favoritos». La atribución debe ser visible también mediante teclado y dispositivos táctiles; no dependerá exclusivamente del hover.

La vista previa incluirá una imagen mayor, dimensiones originales, autor, enlace de origen y licencia. Debe permitir insertar la foto con una acción clara y regresar a la misma posición de búsqueda.

## 5. Incorporación al lienzo

### 5.1 Clic para insertar

1. Resolver de nuevo el recurso si sus referencias han caducado.
2. Preparar la adquisición según la política del proveedor.
3. Registrar el evento exigido por el proveedor, cuando corresponda.
4. Descargar o resolver una representación apta para el motor gráfico.
5. Validar contenido y dimensiones; registrar el recurso en el gestor existente.
6. Crear una capa de imagen centrada en el área visible, conservando proporciones.
7. Guardar la operación como una sola acción de deshacer.

Si falla la preparación no queda una capa rota ni un recurso incompleto. Se permite cancelar una descarga en curso.

### 5.2 Arrastrar y soltar

- Sobre el lienzo: crear una capa en la posición de destino.
- Sobre un marco: rellenarlo con ajuste inicial `cover`, recorte centrado y edición posterior del encuadre.
- Sobre un marco ocupado: sustituir su recurso manteniendo geometría y estilo; deshacer restaura el estado anterior.
- El arrastre transporta una referencia interna del recurso; no confía en HTML remoto.

### 5.3 Resolución y exportación

La miniatura no se usará como fuente definitiva de exportación. Se seleccionará una variante adecuada al tamaño de colocación y a la salida prevista. Para impresión se calculará la resolución efectiva y se advertirá si es insuficiente, sin afirmar que un escalado artificial recupera detalle.

La exportación debe verificar que los recursos estén disponibles y que su acceso sea compatible con la política del proveedor. Un fallo de red no debe producir un archivo aparentemente correcto con imágenes omitidas.

## 6. Almacenamiento y portabilidad

### 6.1 Pixabay

Las miniaturas remotas se emplean para explorar. Al insertar se adquiere una copia para los recursos del proyecto, sin mantener un enlace permanente al CDN como única fuente. En escritorio se propone guardarla en el proyecto; antes de publicar se verificará que el flujo local encaje con los términos y la indicación de descarga a servidor de la documentación.

### 6.2 Unsplash

El catálogo debe utilizar las URL `photo.urls` sin eliminar parámetros requeridos. La selección del usuario invoca `photo.links.download_location`; cargar una miniatura no constituye por sí mismo una selección.

**Dependencia de lanzamiento:** confirmar y documentar un flujo compatible para exportación rasterizada, recursos embebidos en `.gaf`, edición offline y atribución. No asumir que una licencia general de descarga anula los requisitos de hotlinking de la API. Hasta resolverlo, se puede implementar el adaptador en pruebas, pero no anunciar portabilidad offline para esta fuente.

### 6.3 Pexels

Implementar adquisición y persistencia según los términos vigentes y la aprobación del caso de uso. La existencia de una URL descargable no bastará para asumir autorización de redistribución. El proveedor permanecerá deshabilitado si no hay credenciales válidas.

### 6.4 Reglas comunes

Guardar junto a cada recurso: proveedor, identificador, autor, página original, licencia, fecha de incorporación y política aplicada. En `.gaf` utilizar los mecanismos existentes de recursos y metadatos; no inventar un contenedor paralelo.

Los favoritos conservan referencias, no desencadenan descargas de originales. Si una foto deja de estar disponible, indicarlo y ofrecer quitar el favorito. Los recursos ya adquiridos conforme a la licencia se gestionan según la política documentada, sin borrados automáticos injustificados.

## 7. Arquitectura propuesta

Separar interfaz, consultas de catálogo y adquisición de recursos:

- **Panel Fotos:** presenta resultados y emite búsquedas e intenciones de inserción.
- **Servicio de catálogo:** normaliza consultas, capacidades, paginación y resultados.
- **Adaptadores:** encapsulan endpoints, autenticación, atribución y eventos específicos.
- **Intermediario de Gafira:** protege credenciales compartidas, aplica cuotas y caché autorizada.
- **Gestor de recursos existente:** prepara imágenes para lienzo, guardado y exportación.

No incluir claves compartidas en frontend, repositorio, ejecutable ni archivo `.gaf`. La aplicación de escritorio también debe usar el intermediario cuando se empleen credenciales compartidas de Gafira. Los usuarios finales no necesitan claves API ni cuentas de desarrollador.

### 7.1 Contratos TypeScript orientativos

```ts
type PhotoProviderId = 'pixabay' | 'unsplash' | 'pexels';
type PhotoOrientation = 'landscape' | 'portrait' | 'square';

interface PhotoSearchQuery {
  query?: string;
  locale: string;
  orientation?: PhotoOrientation;
  color?: string;
  order?: 'relevant' | 'popular' | 'latest';
  minWidth?: number;
  minHeight?: number;
  size?: 'small' | 'medium' | 'large';
  category?: string;
  cursor?: string;
  pageSize: number;
}

interface PhotoAsset {
  provider: PhotoProviderId;
  providerAssetId: string;
  width: number;
  height: number;
  description?: string;
  thumbnailUrl: string;
  previewUrl: string;
  sourcePageUrl: string;
  author: { name: string; profileUrl?: string };
  license: { name: string; url: string };
  acquisitionRef: string; // Referencia opaca, sin credenciales.
}

interface ProviderCapabilities {
  orientations: PhotoOrientation[];
  colors: string[];
  supportsHexColor: boolean;
  orders: Array<'relevant' | 'popular' | 'latest'>;
  supportsMinDimensions: boolean;
  supportsSizePresets: boolean;
  supportsSafeSearch: boolean;
  supportsOfflineEmbedding: boolean; // Solo tras validar la política.
}

interface PhotoSearchPage {
  items: PhotoAsset[];
  nextCursor?: string;
  total?: number;
}

interface PhotoProvider {
  id: PhotoProviderId;
  capabilities: ProviderCapabilities;
  search(q: PhotoSearchQuery, signal: AbortSignal): Promise<PhotoSearchPage>;
  resolve(id: string, signal: AbortSignal): Promise<PhotoAsset>;
}
```

El servicio de adquisición será independiente de `search`. Su entrada incluirá recurso, destino y un identificador de operación; su salida será una referencia válida para el gestor gráfico. Los contratos deben validarse también en ejecución y rechazar filtros no compatibles.

### 7.2 API interna orientativa

| Operación | Propósito |
| --- | --- |
| `GET /photo-providers` | Fuentes habilitadas y capacidades |
| `GET /photos/search` | Búsqueda por proveedor, filtros y cursor |
| `GET /photos/:provider/:id` | Resolución de metadatos |
| `POST /photo-acquisitions` | Preparar una inserción y eventos asociados |

No aceptar una URL arbitraria como origen de descarga. Resolver la referencia del proveedor y validar dominios y redirecciones permitidas. Las claves nunca se devuelven al cliente.

## 8. Rendimiento y límites

- Página inicial propuesta de 24 resultados, limitada por las capacidades de cada API.
- Imágenes pequeñas adaptadas al tamaño visual y densidad de pantalla.
- Carga diferida fuera del área visible y dimensiones reservadas en cada tarjeta.
- Virtualización cuando la cantidad acumulada lo justifique, conservando foco y posición.
- Máximo de dos solicitudes de catálogo concurrentes por panel.
- Caché por proveedor, consulta normalizada, idioma, filtros y página; Pixabay requiere 24 horas. En otras fuentes aplicar solo la persistencia permitida.
- Leer cabeceras de cuota y `Retry-After`, cuando existan. Ante 429, detener nuevas consultas durante el período indicado; no rotar claves para evadir límites.
- Reintentos limitados con espera exponencial para fallos transitorios. No repetir automáticamente selecciones ni eventos de descarga sin control de duplicación.
- Una operación de inserción tendrá identificador estable para no crear capas dobles al reintentar. Los eventos externos se emitirán según la semántica de cada API; no se presume entrega exactamente una vez.

Objetivos internos: respuesta visual inmediata al escribir o filtrar, sin bloquear el lienzo; interacción del panel fluida con cientos de resultados acumulados. La latencia de red se medirá por proveedor y no se prometerá como una garantía de Gafira.

## 9. Errores, accesibilidad e idiomas

| Estado | Mensaje o comportamiento |
| --- | --- |
| Cargando | Esqueletos con proporciones reservadas |
| Sin resultados | «No encontramos fotos. Prueba otra búsqueda o quita algunos filtros» |
| Sin conexión | «Necesitas conexión para buscar fotos»; conservar el diseño y sus recursos disponibles |
| Cuota agotada | «Esta fuente alcanzó su límite temporal. Prueba otra fuente» |
| Fuente inhabilitada | Ocultarla del selector; no mostrar una pestaña vacía permanente |
| Foto retirada o URL inválida | «Esta foto ya no está disponible» |
| Inserción fallida | Acción de reintento sin capa incompleta |

Todos los controles deben funcionar con teclado, tener etiquetas accesibles y foco visible. Los estados de carga y error se anunciarán sin interrumpir continuamente a lectores de pantalla. Las acciones de arrastre tendrán una alternativa mediante clic. Textos en español e inglés a través del sistema de internacionalización existente.

## 10. Seguridad y observabilidad

- Autenticar y limitar el intermediario para que no sea un proxy público de consumo ilimitado.
- Verificar esquema de respuesta, tipo real de archivo, tamaño máximo de descarga y dimensiones máximas de decodificación.
- Propuesta inicial: 30 MB y 80 megapíxeles por imagen, ajustables tras medir compatibilidad y memoria.
- Descargar fuera del hilo de interfaz y cancelar de forma segura.
- No registrar claves ni URL firmadas completas. Las búsquedas pueden revelar intereses personales: evitar conservar texto de consultas por defecto.
- Medir latencia, aciertos de caché, errores, cuota restante e inserciones fallidas por proveedor.
- Configurar disponibilidad de proveedores en el servicio sin exigir reinstalar Gafira.

## 11. Criterios de aceptación

1. El usuario abre Elementos → Fotos y ve una fuente habilitada con contenido inicial.
2. Una búsqueda devuelve resultados de la API seleccionada y mantiene autor y procedencia visibles.
3. Cambiar de fuente conserva el texto y elimina solo filtros incompatibles.
4. Respuestas de consultas canceladas no modifican la vista actual.
5. El panel no ofrece orientación cuadrada en Pixabay ni otros filtros no soportados.
6. Las miniaturas se cargan progresivamente sin descargar originales.
7. Un clic crea una sola capa con proporciones conservadas; deshacer la elimina.
8. Soltar sobre un marco lo rellena; reemplazar y deshacer restaura el recurso anterior.
9. Una descarga fallida no deja capas ni archivos incompletos vinculados al proyecto.
10. Una foto de Pixabay seleccionada se guarda mediante el flujo aprobado, sin hotlinking persistente como única fuente.
11. La integración de Unsplash conserva URL y atribución, y emite el evento exigido al seleccionar; su lanzamiento no ocurre con el flujo offline pendiente.
12. Pexels no se anuncia como disponible si no hay clave operativa.
13. Las claves no aparecen en código cliente, ejecutable, registros ni archivos de proyecto.
14. Un 429 detiene peticiones durante la espera aplicable y permite cambiar de fuente.
15. La exportación usa recursos de resolución adecuada y comunica cualquier imposibilidad de completarla.
16. Favoritos y Usadas en este proyecto no implican una copia masiva del catálogo.
17. El flujo completo funciona con teclado y en español e inglés.

La validación incluirá pruebas de adaptadores con respuestas simuladas, fallos de red y cuotas; pruebas de integración del gestor de recursos; y recorridos de búsqueda, inserción, marco, deshacer, guardado y exportación. Las comprobaciones con API real serán limitadas y nunca descargarán catálogos completos.

## 12. Orden de implementación

1. Revisar los puntos de extensión reales del panel, recursos, historial y `.gaf`.
2. Implementar contratos, capacidades y servicio intermediario.
3. Completar Pixabay y validar búsqueda → inserción → guardado → reapertura → exportación.
4. Añadir favoritos, historial de proyecto, accesibilidad y manejo de cuotas.
5. Implementar Unsplash en pruebas y resolver sus condiciones de producción y portabilidad.
6. Activar Pexels cuando las credenciales y condiciones lo permitan.

## 13. Fuentes oficiales

Consultadas durante la investigación del 21 de noviembre de 2026. Si cambian sus condiciones, actualizar la política del adaptador y esta especificación antes de activar funciones afectadas.

- Pixabay — API: https://pixabay.com/api/docs/
- Pixabay — resumen de licencia: https://pixabay.com/service/license-summary/
- Pixabay — términos: https://pixabay.com/service/terms/
- Unsplash — API: https://unsplash.com/documentation
- Unsplash — directrices: https://help.unsplash.com/en/articles/2511245-unsplash-api-guidelines
- Unsplash — hotlinking: https://help.unsplash.com/en/articles/2511271-guideline-hotlinking-images
- Unsplash — términos API: https://unsplash.com/api-terms
- Unsplash — licencia: https://unsplash.com/license
- Pexels — API: https://www.pexels.com/api/documentation/
- Pexels — nuevas claves: https://help.pexels.com/hc/en-us/articles/900004904026-How-do-I-get-an-API-key
- Pexels — cuotas y elegibilidad: https://help.pexels.com/hc/en-us/articles/900005852323-How-do-I-get-unlimited-requests
- Pexels — licencia: https://www.pexels.com/license/
