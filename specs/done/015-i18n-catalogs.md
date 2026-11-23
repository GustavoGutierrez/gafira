# 015 — `gafira-i18n`: catálogos es/en, regla de D-08 y traducción por clave

> **Estado:** done — 2026-11-16 — S15-01..S15-12 cubiertos por `gafira-i18n` (`resolver::tests` 3, `catalog::tests` 5, `format::tests` 2, `translator::tests` 14, `pseudo::tests` 3: 27 tests); 9 mutantes a mano, 8 muertos y 1 equivalente; 2718 tests en el workspace. Fechas (zona horaria) y la región en `Translator::text` quedan como seguimiento (ver «No alcance» y «Riesgos»).

## Objetivo

Implementar los puertos de idioma de `gafira-ports::i18n` (02 §4.16, §14) en el adaptador `gafira-i18n`: resolver el idioma de la interfaz según D-08 (`D08LocaleResolver`), traducir mensajes por clave con parámetros y plurales desde catálogos español e inglés embebidos (`CatalogTranslator`), con la cadena de respaldo de §14.6 y la seudo-localización para pruebas (`PseudoTranslator`). Los números siguen D-64.

## Alcance / no alcance

Alcance:

- Catálogos `assets/i18n/es.toml` y `assets/i18n/en.toml` (sintaxis de 02 §14.5): textos con `{parámetros}` y plurales por categoría CLDR. Contenido inicial pequeño: selector de idioma (03 §16.1), eslogan (D-09), el ejemplo de §14.5, el patrón de porcentaje, unidades, tiempo relativo y las claves `error.theme.*` de la spec 014.
- Generación en build: `build.rs` valida los TOML (claves de 02 §14.2, los dos idiomas con las mismas claves y los mismos `{parámetros}`, `other` en cada plural) y genera tablas estáticas ordenadas (`&'static [(&'static str, Entry)]`, búsqueda binaria) que el crate incluye con `include!`. `toml` es solo dependencia de build: en release no hay parser de catálogos ni lectura de disco (ADR-09). Sustituye a `xtask gen-i18n` hasta que exista `xtask`.
- `D08LocaleResolver::new(Arc<dyn SystemLocaleSource>)` implementa `LocaleResolver` con la tabla de 02 §14.4 y la región de formato de D-64.
- `CatalogTranslator` implementa `Translator`: `text` y `has` con parámetros, plurales (`icu_plurals`) y un nivel de `Key`; `number` y `quantity` con `icu_decimal` y la región de D-64; `relative` con mensajes del catálogo con plural.
- Cadena de respaldo (§14.6, ARQ-99): idioma pedido → inglés → en desarrollo `⟦clave⟧`; con la configuración de `dist`, el otro idioma y luego `…`.
- `PseudoTranslator` (feature `pseudo`, ARQ-103): texto inglés acentuado, `{parámetros}` intactos, entre `⟦ ⟧` y alargado con `·` según §14.6.

No alcance (seguimiento):

- **Fechas** (`Translator::date`, ARQ-106 en fecha y hora): `icu_datetime` y, sobre todo, la zona horaria local, que ningún puerto da hoy. Mientras tanto `date` devuelve ISO 8601 en UTC (`2026-03-12`; `Long`: `2026-03-12 14:05 UTC`), igual en los dos idiomas: no es el formato de D-64 y lo sustituirá la spec de fechas.
- «ayer»/«yesterday» en `relative` (depende del día civil local, igual que las fechas).
- ARQ-98 completo (`xtask check-i18n`: extraer todas las claves del código y exigirlas en los catálogos), ARQ-101 (E2E de cambio de idioma), ARQ-103 (recorrido E2E con `PseudoTranslator`), ARQ-104 (CLI) y el tamaño de ARQ-106 (`cargo bloat`).
- `glosario.toml` (03 §2.8), el registro de los catálogos en `assets/LICENSES.toml` (ARQ-75, llega con `xtask`) y la recarga `GAFIRA_I18N_PATH` (feature `dev-reload`).
- El `warn` del log en el respaldo: el workspace aún no tiene log.

## Dependencias

- Puertos de `gafira-ports::i18n` y `gafira-doc::msg`, sin cambiar sus firmas.
- ICU4X 2.4 (Unicode-3.0, ya en la allowlist de `deny.toml`): `icu_locale_core` (ya en `Cargo.lock` por `icu_segmenter`), `icu_plurals` e `icu_decimal` (nuevas). `toml` 1.x (MIT OR Apache-2.0, ya en `Cargo.lock`) solo en `[build-dependencies]`.
- ARQ-107: dependencias normales solo `gafira-ports`, `gafira-doc` y crates ICU4X; ni egui, ni `gafira-engine`, ni `gafira-ui`, ni `gafira-platform`. Como el crate necesita un `build.rs` que lee los catálogos, no lleva el `clippy.toml` de dominio (prohíbe `std::fs`) ni fila V5; un paso propio de CI comprueba sus dependencias directas.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S15-01 | Regla de D-08 exacta (ARQ-100). | Prueba de tabla con un `SystemLocaleSource` falso y ≥ 30 etiquetas de §14.4: cualquier `es*` (`es_CL.UTF-8`, `es_ES@euro`, `ES-mx`, `es-419`) → `Es` con `System { tag }`; `en*` y cualquier otro idioma (incluidos `ca-ES`, `eu-ES`, `gl-ES`) → `En` con `System`; sin dato, ilegible, `C`, `POSIX`, `C.UTF-8` → `En` con `Fallback`; `Es` y `En` manuales dan `Manual` sea cual sea el sistema. |
| S15-02 | Región de formato (D-64). | Interfaz `Es` con formato `es_MX.UTF-8` → `FormatLocale { Es, Some(MX) }`; con formato `fr-FR` → región `None`; interfaz `En` manual con formato `en-GB` → `Some(GB)`; sin formato → `None`. |
| S15-03 | Traducción por clave con parámetros. | `text(Es, msg!("settings.language.system_now", language = Text("Español"), tag = Text("es-MX")))` da «Sistema · ahora: Español (es-MX)»; un `{parámetro}` sin argumento queda literal; `has` es exacto por idioma. |
| S15-04 | Plurales CLDR. | `export.pages_selected` con `Count(1)` → «1 página seleccionada» / «1 page selected»; `Count(3)` → «3 páginas…» / «3 pages…»; `Count(1000000)` en `es` usa `many`. Cada plural de los catálogos tiene todas las categorías que su idioma exige según `icu_plurals` (parte de ARQ-98). |
| S15-05 | Catálogos completos entre sí (parte de ARQ-98). | Las tablas generadas de `es` y `en` tienen las mismas claves, cada clave cumple `is_valid_key` y las dos versiones usan el mismo conjunto de `{parámetros}`. |
| S15-06 | Cadena de respaldo (ARQ-99). | Con catálogos de prueba incompletos: falta en `es` → texto inglés; falta en los dos → `⟦clave⟧` en desarrollo; con la configuración de `dist`, el texto del otro idioma o `…`; ninguna salida de `dist` coincide con `^[a-z0-9_]+(\.[a-z0-9_]+)+$`. |
| S15-07 | Números por idioma y región (D-64, parte de ARQ-106). | `number` `Decimal`: `es` → `1,5`, `1234,5`, `12.345,5`; `es-MX` → `1,234.5`; `en` → `1.5`, `1,234.5`, `12,345.5`. `Percent` usa el patrón del catálogo (`50 %` / `50%`). `Tabular` fija dos decimales. Valores no finitos → `∞`, `-∞` o `NaN`. |
| S15-08 | Cantidades. | `quantity`: `0.5 s` → `0,5 s` / `0.5 s`; `1080 px`; `1200000` bytes → `1,2 MB` / `1.2 MB`; `-14 LUFS`; el espacio entre número y unidad es U+00A0. |
| S15-09 | Tiempo relativo. | `relative` de 30 s → «ahora mismo» / «just now»; 2 h → «hace 2 h» / «2 h ago»; 1 día → «hace 1 día» / «1 day ago». |
| S15-10 | Argumentos tipados en el texto. | `Count`, `Int` y `Decimal` se formatean con `number`; `Quantity` con `quantity`; `Elapsed` con `relative`; `Timestamp` con `date`; `Key` se traduce en el mismo idioma (un nivel). |
| S15-11 | Seudo-localización (ARQ-103, parte unitaria). | `PseudoTranslator` da `⟦…⟧` con letras acentuadas, conserva `{parámetros}` y los sustituye, y alarga según §14.6 (≤ 10 caracteres +100 %, 11-20 +60 %, 21-50 +40 %, > 50 +30 %). |
| S15-12 | Interim de fechas declarado. | `date` devuelve ISO 8601 UTC (`0` → `1970-01-01`; `Long` añade `HH:MM UTC`), igual en los dos idiomas. |

## TDD

Lógica crítica: la regla de D-08 (S15-01, S15-02), el parsing y la validación de catálogos (S15-05), plurales y respaldo (S15-04, S15-06), y la sustitución de parámetros (S15-03, S15-10). Tests que fallan primero: `resolver::tests::s15_01_*`, `s15_02_*`; `translator::tests::s15_03_*` a `s15_12_*`; `catalog::tests::s15_05_*`.

## Evidencia esperada

- Gates de `AGENTS.md`, `cargo deny check` y los pasos de política de CI en verde.
- Banner a `done`, fila en `specs/README.md`.

Resultado (2026-11-16, rama `feat/014-015-theme-i18n`):

- `cargo fmt --check`: ok. `cargo clippy --workspace --all-targets --locked -- -D warnings` y `cargo clippy -p gafira-i18n --all-targets --features pseudo --locked -- -D warnings`: 0 avisos. `cargo test --workspace --locked`: 2718 pasan, 0 fallan.
- Dependencias nuevas en `Cargo.lock`: `icu_plurals` 2.4.0, `icu_plurals_data` 2.4.0, `icu_decimal` 2.4.0, `icu_decimal_data` 2.4.0 y `fixed_decimal` 0.7.3, todas Unicode-3.0. `cargo deny check licenses bans sources`: ok; `advisories` necesita red y queda para la CI.
- Paso nuevo de CI «gafira-i18n depends only on ports, doc and ICU4X (ARQ-107)» en verde; control negativo: añadir `gafira-color` a las dependencias lo hace fallar. V14 falla por un motivo ajeno a esta spec (ver spec 014).
- Mutantes a mano: muertos los de la regla `es`, el filtro de región de D-64, el corte de `.codeset`/`@modificador`, la cadena de respaldo (sin inglés y con el modo `dist` invertido), el idioma de las reglas de plural, la sustitución de argumentos y la forma `many`. Equivalente: la comprobación explícita de `C`/`POSIX`, porque `icu_locale_core` ya rechaza esas etiquetas; se mantiene como guarda.
- Rojo: las pruebas y la implementación entraron en la misma unidad de trabajo; el fallo de las pruebas ante la lógica rota se comprobó con los mutantes de arriba, no con un stub previo.
- Redondeo: `fixed_decimal::Decimal::round` es *half-even*, el modo por defecto de ICU (`12.25` → `12.2`); se adopta y se documenta en la prueba.

## Riesgos y decisiones abiertas

- **`Translator::text` recibe `Locale`, no `FormatLocale`**: los números dentro de un mensaje (`Count`, `Decimal`, `Quantity`) usan el formato CLDR del idioma sin región, así que D-64 solo se cumple en las llamadas directas a `number` y `quantity`. Cerrarlo exige cambiar la firma del puerto (por ejemplo `text(&FormatLocale, &Message)`) o dar al traductor la región resuelta: decisión del titular, fuera de esta spec.
- **Zona horaria**: ningún puerto da el desfase local; hace falta para `date` y para «ayer» (ver «No alcance»).
- **Signo menos y espacio fino**: CLDR da `-` (U+002D) para `es` y `en`, no `−` (U+2212) como la tabla de §14.5; el porcentaje español usa U+00A0 y no el espacio fino U+202F de §14.5, porque las fuentes embebidas hoy (spec 014) no tienen U+202F ni U+2212. Se revisa con las fuentes de producto.
- **Reglas de redondeo** (PROPUESTA): `Decimal` hasta 2 decimales sin ceros finales, `Percent` hasta 1 decimal con el valor en puntos porcentuales (`50.0` → `50 %`), `Tabular` 2 decimales fijos, bytes en múltiplos de 1000 con hasta 1 decimal.
