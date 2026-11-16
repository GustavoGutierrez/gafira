# Herramientas y puertas de calidad

## Puertas locales

Cada commit debe pasar las puertas que cubren lo que puede romper, por sí solo, no solo el último commit de una rama:

```bash
cargo fmt --check
cargo clippy -p <crate> --all-targets --locked -- -D warnings
cargo test -p <crate> --locked
```

El alcance es el crate que toca el commit. Se usa `--workspace` cuando el commit toca un crate del que dependen otros, un `Cargo.toml`, un `Cargo.lock` o `ci.yml`. La suite completa del workspace se ejecuta una vez al cerrar la spec y una vez antes del pull request de `develop` a `main` (ADR 0017).

El toolchain está fijado en `1.93.1` con `rustfmt` y `clippy` en `rust-toolchain.toml`. `--locked` mantiene a `Cargo.lock` como autoridad.

## Lints del workspace

Se definen en el `Cargo.toml` raíz y cada crate los hereda con `[lints] workspace = true`:

| Lint | Nivel |
|---|---|
| `unsafe_code` (rustc) | `deny`; además cada crate declara `#![forbid(unsafe_code)]` |
| `clippy::dbg_macro`, `clippy::todo`, `clippy::unimplemented` | `deny` |

No se debe usar `#[allow]` para silenciar lints. Los crates de dominio añaden un `clippy.toml` con las prohibiciones ARQ-02 y ARQ-10 descritas en el capítulo de arquitectura.

## Perfil release

`opt-level = 3`, LTO ligero (thin), `codegen-units = 4`, `panic = "unwind"`, información de depuración de tablas de líneas, debuginfo dividido y empaquetado, y símbolos eliminados. Las dependencias se compilan con `opt-level = 2` en el perfil de desarrollo.

## CI

**Estado actual.** La CI está desactivada manualmente en GitHub por un problema de facturación, por lo que ningún workflow se ejecuta en pushes ni en pull requests. Las puertas locales son la única barrera: antes de fusionar el trabajo en `develop` se ejecutan en local los tres comandos anteriores, `cargo deny check` y los pasos de `policy` de `ci.yml` (ARQ-07, prohibición de `unsafe`, V5, V12, V13), los pasos bajo `bash --noprofile --norc -eo pipefail`, y se registran en la sección de evidencia de la spec. Por tanto, las pruebas en Windows y macOS de la matriz no se ejercitan hoy. La descripción siguiente corresponde al archivo de workflow tal como está en el repositorio, a la espera de que el propietario lo reactive.

El workflow es `.github/workflows/ci.yml`. Se ejecuta en pushes a `main`, pull requests y ejecución manual, con permiso `contents` de solo lectura y cancelación en curso para los pull requests. Las acciones de terceros se fijan con SHA de commit de 40 caracteres hexadecimales. Su tabla V5 tiene una fila por cada crate de biblioteca, incluido `gafira-testkit`.

| Job | Se ejecuta en | Pasos |
|---|---|---|
| `test` | Matriz: `windows-2022` (`x86_64-pc-windows-msvc`), `ubuntu-24.04` (`x86_64-unknown-linux-gnu`), `macos-14` (`aarch64-apple-darwin`) | Instalar el toolchain fijado; comprobar que el triple del host coincide con el destino de la matriz; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked` |
| `policy` | `ubuntu-24.04` | `cargo fmt --check`; la CLI no tiene dependencias de UI (ARQ-07); todo `lib.rs` y `main.rs` tiene `#![forbid(unsafe_code)]`; tabla de capas (V5); copias de `clippy.toml` sincronizadas (V12); `EditToken` solo en los crates de edición (V13); `cargo-deny check` |

El job `policy` es donde las reglas de arquitectura se vuelven ejecutables. Véase el capítulo de arquitectura para V5, V12 y V13.

## Política de dependencias y licencias

`deny.toml` gobierna `cargo-deny`:

- Licencias: solo permisivas (MIT, Apache-2.0 con o sin la excepción LLVM, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unicode-DFS-2016, BSL-1.0, CC0-1.0, MIT-0, 0BSD). Los crates del workspace usan `LicenseRef-Gafira-Proprietary` y son privados (`publish = false`). Los cambios a la lista permitida para dependencias transitivas requieren una entrada `[[licenses.exceptions]]` por crate con una justificación escrita.
- Bans: se deniegan las versiones con comodín (las dependencias por ruta están exentas); las versiones duplicadas avisan. Una lista de denegación bloquea los bindings de FFmpeg, `symphonia`, los crates de colecciones persistentes (`imbl`, `im`, `rpds`), `gstreamer`, `openssl-sys`, los codificadores con patentes o LGPL (`openh264`, `x264`, `mp3lame-encoder`) y los envoltorios de GNU gettext.
- Fuentes: se deniegan los registros y las fuentes git desconocidos.
- Avisos de seguridad: se deniegan los crates retirados (yanked). La base de datos de avisos necesita red, por lo que se ejecuta en la CI y puede omitirse en local.
- Destinos comprobados: los tres triples soportados.

## Política de pruebas

- Las pruebas cubren el comportamiento del contrato público, no la implementación.
- Capas de prueba: pruebas unitarias junto al código, pruebas de integración en el directorio `tests/` de cada crate, pruebas de propiedades con `proptest` (dependencia del workspace con características por defecto desactivadas y `std` activada) pruebas compile-fail con `trybuild` y archivos golden para el formato de documento. Aún no existe una capa de extremo a extremo.
- Todo comando del núcleo pasa por el arnés `prop_command` de seis leyes de `gafira-testkit` (véase más abajo y el capítulo de edición).
- La lógica crítica se construye con la prueba primero; el capítulo de metodología la enumera (modelo de documento, carga, guardado y validación de JSON, ids y arena, historial de edición, tiempo, color, geometría y transformaciones, límites de tamaño y DPI, parsing de formatos y toda regla con invariante). No crítico: el texto de `Display` y `Error` salvo que una spec lo fije, los accesores triviales y los ayudantes de prueba.
- Las doctests compile-fail vigilan reglas de capacidad (por ejemplo, que `EditToken` no sea `Clone`). Son ayudas de regresión, no una prueba; la evidencia provisional es el paso V13 de la CI más los fixtures de `trybuild`.

### gafira-testkit

Un crate de soporte (solo dev-dependency, `publish = false`, fila V5, sin `clippy.toml`). Ofrece:

- `document()`: una estrategia de documentos válidos, acotada (como máximo 3 páginas, con número de nodos y profundidad de árbol limitados), o el documento vacío.
- `corrupted()`, `corrupt(doc, Corruption, pick)` y el enum `Corruption` de 24 variantes: un documento válido roto de exactamente una manera, con el error de validación que debe producir.
- `prop_command` y `try_prop_command`: el arnés de seis leyes para comandos (1 000 casos por defecto; la variable de entorno `PROPTEST_CASES` lo modifica).

Úselo solo desde pruebas de integración (`tests/`), donde cada crate tiene una copia de sus tipos. El generador nació en las pruebas de `gafira-doc` y se movió aquí para que `gafira-edit` lo comparta.

### trybuild

`trybuild` (dev-dependency de `gafira-edit`) compila los fixtures de `crates/gafira-edit/tests/ui/` y compara la salida del compilador con los archivos `.stderr` guardados. Esos archivos citan texto del compilador, así que un cambio de la versión fijada del toolchain o de una firma citada exige regenerarlos con `TRYBUILD=overwrite` y revisarlos. Los fixtures son la evidencia provisional de ARQ-30; el capítulo de edición explica qué demuestran.

## Pruebas de mutación

`cargo-mutants` es una herramienta opcional para desarrolladores, como `cargo-deny`; no es una puerta de la CI y no tiene objetivo de puntuación. Se usa solo para la lógica crítica (véase el [capítulo de metodología](10-methodology.md)), limitada a un crate:

```bash
cargo mutants -p <crate> --jobs 8 -o <scratch dir>
```

Un mutante superviviente en lógica crítica indica una laguna de pruebas, salvo que sea equivalente y esté excluido en `.cargo/mutants.toml` (véanse las reglas siguientes). Una mutación manual es una comprobación más ligera: se cambia una línea a mano (por ejemplo `<` en lugar de `<=`, sin `break` temprano), se ejecutan las pruebas, se comprueba que una falla y se restaura el código. Así se demuestra que una prueba escrita después del código puede fallar. Los comandos y resultados de cualquiera de las dos comprobaciones se registran en la sección de evidencia de la spec.

Reglas, de `AGENTS.md` y `.cargo/mutants.toml`:

- Use el ejecutor `cargo test` por defecto (nextest omite las doctests) y no use `RUSTFLAGS=-D warnings`.
- `--locked` ya viene de `additional_cargo_test_args` en `.cargo/mutants.toml`; no lo pase de nuevo, porque cargo rechaza un duplicado.
- `.cargo/mutants.toml` fija `profile = "mutants"`. La tabla `[profile.mutants]` de `Cargo.toml` hereda `test` y fija `debug = "none"`, lo que acorta la compilación de cada mutante. No pase `--profile`.
- Use `--jobs 8`. Un enlazador rápido como mold o wild es opcional y no está instalado.
- `#[mutants::skip]` está prohibido porque requiere una dependencia. Los mutantes equivalentes se excluyen en `.cargo/mutants.toml` con una expresión regular que coincide con exactamente un mutante y una razón escrita. La entrada actual excluye `Document::schema_version -> u32 with 1`, equivalente mientras `SCHEMA_VERSION` sea 1 y la carga acepte solo la versión 1.
- Dirija la salida a un directorio temporal con `-o`; `/mutants.out/` está ignorado por git.

## Tamaño de las entregas

Mida cada entrega con `git diff --shortstat`. Divídala cuando supere unas 700 líneas; las estimaciones se han quedado cortas entre 1,4 y 1,9 veces, sobre todo en pruebas. Las entregas se cortan en límites de commit y cada prefijo debe pasar las puertas por sí solo.
