# gafira-ports: los puertos y sus registros

## Estado

| Área | Estado |
|---|---|
| Traits de puerto en los trece módulos del crate | Implementado |
| `Registry<T>` y `RegistryError` | Implementado |
| Un enum de error por familia de puertos (ADR 0014) | Implementado |
| `PluginHost` | **Planificado**: llega con `gafira-plugin` (`specs/02-arquitectura-software.md:825-833`) |
| `LibraryRepository`, `SettingsRepository` | **Planificado**: `specs/02-arquitectura-software.md:1343` |
| Trait de fuente de fotogramas para el tipo `VideoNode` | **Planificado**: `docs/adr/0015-layer-3-sibling-seams.md:23` |
| `Project`, `AssetRef` y `AssetTable` como raíz del modelo unificado | **Planificado**: `specs/02-arquitectura-software.md:199-215` (ARQ-55); hoy `ProjectRepository` carga y guarda un `Document` |

## Propósito

Traits de puerto y registros de la capa 3: solo traits y datos simples, sin I/O y sin estado
global. Los adaptadores los implementan y el motor los consume
(`crates/gafira-ports/src/lib.rs:1-5`). El crate declara trece módulos públicos y no
reexporta nada en su raíz, de modo que cada elemento se alcanza por la ruta de su módulo
(`lib.rs:5`, `lib.rs:9-21`).

| Aspecto | Valor |
|---|---|
| Dependencias internas | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-time`, `gafira-raster` |
| Dependencias externas | `serde`, `kurbo` |
| `unsafe` | Prohibido por `#![forbid(unsafe_code)]` (`lib.rs:7`) |
| Fila de la capa 3 en la tabla del workspace | `specs/02-arquitectura-software.md:100`; aristas fijadas por el ADR 0013 |

El conjunto de aristas es el que registra el ADR 0013 y la CI lo verifica: la fila de V5 dice
`gafira-ports gafira-geom gafira-color gafira-doc gafira-time gafira-raster`
(`.github/workflows/ci.yml:107`). ARQ-02 no obliga a la capa 3, pero el crate mantiene un
`clippy.toml` idéntico byte a byte a la copia canónica, que prohíbe `std::fs`, `std::net`, la
creación de hilos, `std::time::SystemTime::now`, `std::env`, `std::process`, `HashMap` y
`HashSet` (`crates/gafira-ports/clippy.toml:6-62`; ARQ-10; decisión 2 del ADR 0013).

Las reglas de capas y la puerta V5 que lee las aristas de dependencia de este crate están
documentadas en `02-architecture.md`.

## Mapa de módulos

| Módulo | Puertos y tipos | Registro |
|---|---|---|
| `assets` | `AssetProvider`, `AssetQuery`, `AssetPage`, `AssetItem` | `AssetRegistry` |
| `assist` | `TextAssistant`, `RewriteRequest`, `AssistReq`, `Suggestion` | `TextAssistantRegistry` |
| `common` | `Confidence`, `LegalClass`, `Bytes`, `UnixTime` | ninguno |
| `export` | `Exporter`, `ExportSink`, `PageRenderer`, `ExportCaps` | `FormatRegistry` |
| `format` | `Importer`, `FormatDescriptor`, `ByteSignature`, `Imported` | `FormatRegistry` |
| `i18n` | `SystemLocaleSource`, `LocaleResolver`, `Translator`, `Locale` | ninguno |
| `license` | `LicenseVerifier`, `LicenseStatus`, `LicenseKey` | ninguno |
| `media` | `Container`, `Demuxer`, `Muxer`, `Codec`, `Decoder`, `Encoder` | `CodecRegistry` |
| `registry` | `Registry<T>`, `RegistryError`, `Describe` | la base genérica |
| `render` | `RenderBackend`, `TileCache`, `Effect`, `Transition`, `RenderOp` | `EffectRegistry`, `TransitionRegistry` |
| `repository` | `ProjectRepository`, `LoadedProject` | ninguno |
| `store` | `ContentStore` | ninguno |
| `system` | `FileSystem`, `Clock`, `Notifier`, `ReadSeek`, `WriteSeek`, `FileWrite` | ninguno |

## El registro

Cada familia que los adaptadores extienden es un `Registry<T>`; el crate no incluye ninguno
global (ARQ-31, `crates/gafira-ports/src/registry.rs:37-38`), así que la raíz de composición
posee cada registro y lo pasa hacia abajo.

```rust
// crates/gafira-ports/src/registry.rs:40-43
pub struct Registry<T: ?Sized> {
    items: Vec<Arc<T>>,
    by_id: BTreeMap<&'static str, usize>,
}
```

| Elemento | Contrato |
|---|---|
| `new`, `Default` | Un registro vacío; funciona para `T` sin `Describe` |
| `register(item)` | Exige `T: Describe`; guarda el elemento y lo indexa por su id estático |
| `get(id)` | El elemento registrado para un id, o `None` |
| `iter` | Los elementos en orden de registro, que es el que recorre el descubrimiento |
| `len`, `is_empty` | Cantidad de elementos registrados |

`register` devuelve `Result<(), RegistryError>` y su única variante es
`DuplicateId { id: &'static str }`. Un id duplicado se rechaza y el registro queda intacto
(`registry.rs:80-83`, `registry.rs:13-16`). El orden de registro forma parte del contrato
porque el descubrimiento de formatos y el desempate de códecs lo leen.

## Puertos de renderizado

`RenderBackend` es el límite con una implementación de rasterizado. Informa su `BackendCaps`
(`kind` CPU o GPU, textura máxima, formatos admitidos y si ejecuta efectos), abre un
fotograma sobre un `TargetDesc`, ejecuta un `RenderPlan`, lee píxeles y expone su propia
contabilidad de memoria:

| Elemento | Contrato |
|---|---|
| `caps` | Capacidades estáticas del adaptador (render.rs:282) |
| `begin_frame(target)` | Un `FrameCx` para el destino, o un `RenderError` |
| `execute(cx, plan, cache, deadline, cancel)` | Un `RenderReport`, nunca un fallo parcial |
| `read_back(cx, region)` | Un `ImageBuf` de la región pedida |
| `memory_usage()` | `MemoryUsage { host, device }` en bytes |
| `trim(budget)` | Libera memoria hasta el presupuesto (ARQ-44) |

El contrato de tile, región y origen es la parte que un backend no puede improvisar, y
ARQ-149 lo fija: los valores de `RenderOp` quedan en coordenadas de página, `TileTask.region`
es un `IRect` semiabierto en píxeles de página ya recortado al viewport de la petición, y
`TargetDesc.origin` es la esquina superior izquierda de la superficie en píxeles de página
(`crates/gafira-ports/src/render.rs:100-105`, `render.rs:150-156`). El backend, por lo tanto,
nunca necesita conocer el tamaño de la página, y un tile nunca depende de dónde esté la
superficie.

`TileKey` es la clave de contenido: un `ContentKey(u128)` del contenido renderizado más nivel,
columna y fila, así que la caché de tiles está direccionada por contenido (ARQ-142,
`render.rs:31-45`). `TileCache` es el adaptador sobre el que el backend escribe: `get`, `put`,
`bytes` y `trim(budget)`, todo en bytes, que es el contrato del gobernador de ARQ-44
(`render.rs:112-121`).

Cancelación y demora son cosas distintas. `execute` recibe un `CancelToken` e informa la
cancelación como `RenderError::Cancelled`, comprobada al menos una vez por tile y por
fotograma (ARQ-139); un fotograma que no pudo terminar dentro de su `Deadline` no es un error
sino un `RenderReport { done, left }` cuyo `is_complete` es falso (`render.rs:195-210`,
`render.rs:291-294`). `read_back` devuelve píxeles `Rgba8` o `RgbaF32` y nunca `Alpha8`
(`render.rs:155-157`, `render.rs:308`). La recuperación ante un dispositivo o adaptador
perdido vive en las variantes del error — `DeviceLost`, `OutOfMemory`, `AdapterMissing`,
`DriverBlocked` —, que es como ARQ-22 degrada de GPU a CPU sin tocar el documento.

`PageRenderer` es el puerto acotado que se usa fuera de la tubería interactiva: recibe un
`PageRenderRequest` (página, tiempo, escala, región, transparencia) y devuelve un `ImageBuf`
compuesto, informando la cancelación como `RenderError::Cancelled` (`export.rs:129-142`).
`Effect` y `Transition` son las extensiones enchufables del renderizado; sus registros son
`Registry<dyn Effect>` y `Registry<dyn Transition>` (`render.rs:24-27`).

## Puertos de formato

`Importer` y `Exporter` comparten un `FormatDescriptor` (`id`, extensiones, MIME, firmas de
bytes, `FormatKind`, `LegalClass`). Un importador añade `sniff`, que devuelve una
`Confidence`, e `import`, que lee de un `ReadSeek` y devuelve un `Imported` como activo, nodos
o páginas (`format.rs:166-177`; FD-31). Un exportador declara `capabilities`,
`options_schema`, `validate` y `export` (`export.rs:225-243`).

| Puerto | Qué debe garantizar |
|---|---|
| `ExportCaps` | Declara transparencia, multipágina, animación, CMYK, sangrado y notas; `contains` responde a un conjunto de requisitos |
| `ExportIssue`, `IssueSeverity` | Validación previa a la escritura; `blocks()` separa una parada dura de un aviso (FD-58) |
| `ExportScope` | Página, selección o rango (FD-57) |
| `ExportSink` | `create(name)` entrega un `WriteSeek`; el destino es del llamador |
| `Exporter` | Vive en un adaptador y nunca ve `gafira-render` (`export.rs:4-6`, ARQ-06) |

`FormatRegistry` es un solo registro para ambas direcciones, con `register_importer`,
`register_exporter`, `importers`, `exporters`, `exporters_with` y las entradas de
descubrimiento `resolve_import` e `import` (`format.rs:191-330`). El descubrimiento implementa
las reglas 1 a 6 de la sección 4.5 del documento de arquitectura: extensión, última
extensión, plegado de mayúsculas, olfateo de firmas mediante `ByteSignature::matches` y el
orden de confianza, con iteración ordenada para que el resultado sea determinista.

## Puertos de medios

Los medios se escriben como una tubería de envío y recepción para que ningún adaptador posea
un hilo: `Container` sondea una cabecera, abre un `Demuxer` o crea un `Muxer`; `Demuxer`
enumera flujos, lee `Packet`s y busca; `Muxer` añade flujos, escribe paquetes y termina con un
`MuxReport`; `Codec` construye un `Decoder` o un `Encoder`, y ambos intercambian `send` y
`receive` (`crates/gafira-ports/src/media.rs:293-404`).

`CodecRegistry::best_decoder` codifica la política de licencias de D-02: un códec gravado nunca
se selecciona, y entre los demás el orden es hardware, luego con licencia del sistema, luego
libre de regalías, con el orden de registro como desempate (`media.rs:448-452`). ARQ-77 —el
binario no lleva códec propio de H.264, HEVC ni AAC— es lo que la política protege.

## Puertos de plataforma

| Puerto | Qué exige |
|---|---|
| `FileSystem` | `open`, `create`, `replace`, `remove`, `exists`, `create_dir_all`, `read_dir`, todo `std::io::Result` |
| `FileSystem::replace` | Renombrado atómico más sincronización del directorio, el mecanismo detrás de ARQ-50 y del paso 3 de la §7.2 (system.rs:47) |
| `Clock` | `now` como `UnixTime`; `monotonic` como un `Duration` que nunca retrocede |
| `Notifier` | `notify(title, body)` con el texto ya traducido, con esfuerzo de mejor intento |
| `ReadSeek`, `WriteSeek`, `FileWrite` | Traits de flujo con implementaciones blanket sobre `std`; `FileWrite` añade `sync_all` |

`Clock` es la única fuente de tiempo para la aplicación (ARQ-138) y su mitad monótona es
contra la que se mide `Deadline`; las pruebas inyectan un reloj falso para que las
transiciones de estado sean deterministas (ARQ-93). `Notifier` es deliberadamente débil: no
lleva idioma ni error, porque el mensaje llega ya formateado por `Translator` (D-08).

## Puertos de documento, contenido y activos

`ProjectRepository` carga y guarda un documento por ruta. `load` devuelve un `LoadedProject`,
que lleva la marca `read_only`: un archivo escrito por un esquema más nuevo se carga en solo
lectura y nunca se sobrescribe, y `RepositoryError::TooNew { found, supported }` informa el
par de versiones (`crates/gafira-ports/src/repository.rs:14-23`, `repository.rs:68-83`;
ARQ-55). Mientras no llegue el modelo unificado, la carga útil es un `Document` y no un
`Project`.

`ContentStore` es el almacén de bytes direccionado por contenido: `put` devuelve un
`ContentHash`, `open` devuelve un `ReadSeek` y `contains` responde sin leer; `StoreError` es
`NotFound` o `Io` (`store.rs:14-71`). `AssetProvider` es el puerto de biblioteca: `kinds`,
`search` sobre un `AssetQuery` (desplazamiento y límite, donde cero significa sin límite),
`thumbnail` acotada por el lado mayor y `materialize`, que copia el activo a un `ContentStore`
e informa el hash y el MIME (`assets.rs:126-155`).

## Puertos de idioma, asistencia de texto y licencia

`i18n` fija el vocabulario de idioma de la capa 3 mientras el dominio se mantiene sin idiomas
(ARQ-107): `Locale` es `Es` o `En` y nada más, `LanguagePreference` añade `System` y se
analiza y serializa por su propio nombre, y `SystemLocaleSource::system_locale` es el puerto
por el que la plataforma informa los idiomas de interfaz y de formato del sistema
(`crates/gafira-ports/src/i18n.rs:14-141`; RF-65, RF-66, D-08). `LocaleResolver` convierte
una preferencia en un `ResolvedLocale` con un `LocaleSource` manual, de sistema o de reserva,
y `Translator` formatea texto, números, fechas, tiempos relativos y cantidades para un
`FormatLocale` (`i18n.rs:145-156`; ARQ-100).

`TextAssistant` es el puerto opcional de reescritura local: `requirements` informa RAM, GPU y
tamaño del modelo, y `rewrite` devuelve sugerencias para un tono, una longitud y un idioma,
dados `Progress` y un `CancelToken` (`assist.rs:101-120`). Una sugerencia nunca se aplica
sola —el usuario elige una— (`assist.rs:59`) y el puerto es local, está desactivado por
defecto y queda fuera del MVP (ARQ-26, FD-64). Sus errores son `ModelMissing` y `Cancelled`.

`LicenseVerifier` mantiene la decisión de licencia en local salvo la activación: `status` es
una función pura de `UnixTime` y sin red, `activate` y `validate` son las únicas llamadas que
pueden alcanzar la red, y `deactivate` libera el puesto
(`crates/gafira-ports/src/license.rs:108-133`; FD-05, RNF-04). `LicenseStatus` tiene cinco
valores —`Active`, `OfflineGrace`, `Expired`, `Unlicensed`, `Invalid`— sin estado de prueba
(D-12), y `can_export` es verdadero solo para `Active` y `OfflineGrace` (D-55). `LicenseKey`
tiene un `Debug` redactado y ningún `Display`, así que la clave no puede llegar a un registro
por accidente (ARQ-94, `license.rs:57-79`).

## Errores y cancelación

Cada familia de puertos posee un enum de error, como exige el ADR 0014
(`specs/02-arquitectura-software.md:1590`); los errores de capas inferiores viajan como claves
de mensaje con parámetros (ARQ-102).

| Error | Familia | Variantes |
|---|---|---|
| `RegistryError` | genérico | `DuplicateId` |
| `StoreError` | almacén de contenido | `NotFound`, `Io` |
| `AssetError` | activos | `NotFound`, `Store`, `Io` |
| `RepositoryError` | repositorio | `Corrupt`, `TooNew`, `Io` |
| `ImportError` | importación | `Unsupported`, `Corrupt`, `Store`, `Io`, `Cancelled` |
| `ExportError` | exportación | `Render`, `Media`, `Store`, `Io`, `Cancelled` |
| `RenderError` | renderizado | `DeviceLost`, `OutOfMemory`, `AdapterMissing`, `DriverBlocked`, `Effect`, `Buffer`, `Cancelled` |
| `MediaError` | medios | `UnsupportedCodec`, `Corrupt`, `SystemCodecUnavailable`, `ChannelMismatch`, `Io`, `Cancelled` |
| `LicenseError` | licencia | `Network`, `Rejected`, `Storage` |
| `AssistError` | asistencia | `ModelMissing`, `Cancelled` |

`Cancelled` no se define aquí. Es la unidad de estructura que declara `gafira-doc` en
`crates/gafira-doc/src/contract/cancel.rs:64`, reexportada como
`gafira_doc::contract::Cancelled`, junto al `CancelToken` que transporta la petición y al
trait `Progress` que la informa (`cancel.rs:14`, `cancel.rs:87`). Es el único valor de
cancelación de todo motor (ADR 0012), y los puertos reciben tokens prestados y envuelven el
resultado, de modo que la cancelación cruza un límite de trait sin que el puerto invente su
propia bandera:

```rust
// crates/gafira-ports/src/render.rs:212-224, 248
pub enum RenderError {
    // ...
    Cancelled(Cancelled),
}

impl From<Cancelled> for RenderError {
    fn from(c: Cancelled) -> Self { Self::Cancelled(c) }
}
```

El mismo patrón, con la misma conversión `From<Cancelled>`, existe en `ExportError`
(`export.rs:154`, `export.rs:206`), `ImportError` (`format.rs:120`, `format.rs:147`),
`MediaError` (`media.rs:32`, `media.rs:67`) y `AssistError` (`assist.rs:70`,
`assist.rs:85`). Los métodos que aceptan un token son `RenderBackend::execute`,
`PageRenderer::render`, `Exporter::export`, `Importer::import` y `TextAssistant::rewrite`.

## Pruebas

Las pruebas de integración viven en `crates/gafira-ports/tests/`, un archivo por módulo, y
cada nombre de prueba lleva el identificador de requisito que demuestra.

| Archivo | Contrato fijado |
|---|---|
| `registry.rs` | Registro vacío sin `Describe`, orden de registro, id duplicado rechazado |
| `render.rs` | Backend a través de `dyn`, fotograma demorado es informe, cancelación es error, formatos de `read_back`, presupuesto de `TileCache` |
| `export.rs` | `ExportCaps`, `ExportIssue::blocks`, seguridad de objeto de `Exporter`, `ExportSink`, `PageRenderer` |
| `format.rs` | Tabla de firmas, confianza del olfateo, reglas de extensión y orden, id de importador duplicado |
| `media.rs` | Política de selección de decodificador, conversión de `Cancelled`, desajuste de canales de `AudioBuf`, compatibilidad con `dyn` |
| `system.rs` | Traza del guardado atómico, operaciones de sistema de archivos por `dyn`, `Clock` inyectable, `Notifier` |
| `repository.rs`, `store.rs`, `assets.rs` | Seguridad de objeto por `dyn`, ida y vuelta del documento, errores de almacén y activos |
| `i18n.rs`, `license.rs`, `assist.rs`, `common.rs` | Análisis de preferencias, `can_export`, redacción de la clave, cancelación de reescritura, orden de `Confidence` |

La seguridad de objeto se prueba de forma explícita porque el motor consume cada puerto como
`dyn Trait`; los mensajes de error se fijan contra claves de `ToMessage`, que es como se
aplica ARQ-102 por familia.
