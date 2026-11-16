# 014 — `gafira-theme`: tokens de diseño desde datos y tipografías embebidas

> **Estado:** done — 2026-11-16 — S14-01..S14-12 cubiertos por `gafira-theme` (`color::tests` 3, `error::tests` 1, `fonts::tests` 1, `tokens::tests` 17, `tests/arq_105.rs` 2: 24 tests); 10 mutantes a mano en la validación y la fórmula, 9 muertos y 1 equivalente; 2718 tests en el workspace. Fuentes de producto y escala de sombras oscura pendientes del titular (ver «Riesgos»).

## Objetivo

Que la UI lea sus colores, tipografía, espacios, tamaños, bordes, sombras, movimiento y capas z de un único archivo de datos, `assets/theme/gafira.tokens.json` (contenido normativo en 03 §3), validado al cargar (02 §8.2), y que tenga Inter y Manrope embebidas en el binario (DIS-05). El crate expone tipos neutros, sin egui (02 §2.1, línea de `gafira-theme`).

## Alcance / no alcance

Alcance:

- `assets/theme/gafira.tokens.json` con el JSON de 03 §3, copiado sin cambios.
- `gafira-theme`: `ThemeTokens::embedded()` (el JSON embebido con `include_str!`) y `ThemeTokens::from_json(&str)`; tipos neutros `ThemeTokens`, `Brand`, `ModeTokens`, `SplashTokens`, `TypeTokens`, `TextStyle`, `SpaceTokens`, `RadiusTokens`, `SizeTokens`, `BorderTokens`, `Shadow`, `MotionTokens`, `ZTokens`, `Rgb8`; `ThemeError` con `ToMessage` (ARQ-102).
- Validación al cargar (02 §8.2, pasos 1-5): `schema` conocido, referencias `{ruta}` resueltas y sin ciclos, mismos tokens en los dos modos, contraste WCAG de los pares de `contrast.pairs` con los mínimos de `contrast.min` (DIS-10, ARQ-61), pares de `contrast.rejected` ausentes de `pairs` y por debajo del mínimo de texto, y `motion.reduced` con duraciones 0.
- Tipografías embebidas: Inter y Manrope con su `OFL.txt` (`EmbeddedFont`, `fonts()`); cada familia de `type.family` existe entre las embebidas.
- ARQ-105: prueba de cobertura `cmap` de las dos fuentes embebidas con los caracteres de los catálogos de `gafira-i18n` (spec 015) y la lista de ARQ-105.

No alcance:

- Traducir los tokens a `egui::Style`/`Visuals` (`gafira-ui`, `theme.rs`, ARQ-60) y el cambio de modo Claro/Oscuro/Sistema en Ajustes.
- `GAFIRA_THEME_PATH` (recarga en desarrollo, 02 §8.2): llega con `gafira-ui`/`apps/gafira`; `from_json` ya permite cargar otro archivo.
- Las fuentes de producto completas (ver «Riesgos»): se embeben los subconjuntos ya registrados en `assets/LICENSES.toml`.
- Logo, iconos y `xtask gen-brand` (02 §8.2, §8.4).

## Dependencias

- 03 §3 (contenido de los tokens) y §4 (fórmula de contraste); 02 §8.2 (mecanismo) y §2.1 (capas).
- `gafira-doc::msg` para `ToMessage`. `serde` y `serde_json` (ya en el workspace). `skrifa` solo como `dev-dependency` (ARQ-105).
- Sin dependencias de egui, `gafira-engine` ni `gafira-ui`: fila V5 de CI con `gafira-doc` y sus dependencias.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S14-01 | El JSON embebido carga y es válido. | `ThemeTokens::embedded()` devuelve `Ok`; `brand.ink` es `#18243B`, `modes.light.bg` es `#FAFAF7`, `type.scale.h3.weight` es 650, `space.unit` es 4 y `motion.reduced` vale 0 en las tres duraciones. |
| S14-02 | `schema` conocido (02 §8.2 paso 1). | `schema` 1 carga; `schema` 2 da `SchemaTooNew { found: 2, supported: 1 }`; `schema` 0 da `SchemaUnknown`. |
| S14-03 | Colores bien formados. | Cada color es `#RRGGBB` o una referencia `{ruta}`; `#18243` o `#GG0000` dan `BadColor` con la ruta del token. |
| S14-04 | Referencias resueltas y sin ciclos (paso 2). | `splash.panel = "{modes.light.bg}"` resuelve a `#FAFAF7` y `progress.fill = "{brand.coral}"` a `#FF6B57`; una referencia a una ruta que no existe da `UnresolvedReference`; `a → b → a` da `ReferenceCycle`. |
| S14-05 | Los dos modos tienen los mismos tokens (paso 3). | Quitar `link` de `modes.dark` da `ModeMismatch { token: "link" }`; quitar un token obligatorio de los dos modos da `MissingToken`. |
| S14-06 | Contraste WCAG de los pares declarados (paso 4, DIS-10, ARQ-61). | La fórmula de 03 §4 reproduce los ratios de referencia del manual (14,83; 15,51; 5,53; 2,80; 6,82 con dos decimales); los 84 pares de `contrast.pairs` cumplen su mínimo (los 2 `exempt` no cuentan); cambiar `modes.light.text.muted` a `#8E97AA` da `Contrast` con el par, el modo y el ratio. |
| S14-07 | Pares rechazados (paso 4). | Cada par de `contrast.rejected` queda por debajo de `contrast.min.text`; un par de `pairs` cuyos colores resueltos coinciden con uno de `rejected` da `RejectedPairDeclared`. |
| S14-08 | Movimiento reducido (paso 5). | Sin `motion.reduced` da `MissingToken`; `reduced.fast = 120` da `ReducedMotionNotZero`. |
| S14-09 | Tipos neutros sin valores en código. | El crate no depende de egui; ningún valor de token (color, medida, duración) aparece como literal en `src/` fuera de las pruebas: los nombres de tokens sí, porque son el contrato con la UI. Sombras: `"none"` o `"x y blur rgba(r,g,b,a)"` en px; un texto distinto da `BadShadow`. |
| S14-10 | Tipografías embebidas (DIS-05). | `fonts()` devuelve Inter y Manrope con bytes no vacíos y su texto OFL; `type.family.display` y `type.family.ui` nombran una familia embebida; otra familia da `UnknownFamily`. |
| S14-11 | Las fuentes cubren los dos idiomas (ARQ-105). | La `cmap` de cada fuente embebida contiene todos los caracteres de `assets/i18n/es.toml` y `en.toml` y ¿ ¡ ñ Ñ á é í ó ú ü Á É Í Ó Ú Ü « » “ ” ‘ ’ — – … × °: 0 ausentes. |
| S14-12 | Errores como claves (ARQ-102). | Cada variante de `ThemeError` da una clave `error.theme.<variante>` válida (`is_valid_key`) con sus parámetros. |

## TDD

Lógica crítica: carga y validación del JSON (S14-02..S14-08) y la fórmula de contraste (S14-06). Tests que fallan primero, en `gafira-theme` (`tokens::tests`, `contrast::tests`, `tests/arq_105.rs`):

1. `s14_06_formula_reproduces_the_manual_ratios`.
2. `s14_02_*`, `s14_03_*`, `s14_04_*`, `s14_05_*`, `s14_07_*`, `s14_08_*` sobre variantes del JSON embebido editadas en la prueba.
3. `s14_01_embedded_tokens_load` y `s14_06_every_declared_pair_meets_its_minimum`.

## Evidencia esperada

- Gates de `AGENTS.md` en verde, incluidos los pasos de política de CI (V5, V12, ARQ-107) y `cargo deny check`.
- Banner a `done`, fila en `specs/README.md`.

Resultado (2026-11-16, rama `feat/014-015-theme-i18n`):

- `cargo fmt --check`: ok. `cargo clippy --workspace --all-targets --locked -- -D warnings`: 0 avisos. `cargo test --workspace --locked`: 2718 pasan, 0 fallan.
- `cargo deny check licenses bans sources`: ok (sin dependencias normales nuevas; `skrifa` ya estaba en el workspace). `advisories` necesita red y queda para la CI.
- Pasos de política de CI ejecutados en local: unsafe, V5 (con la fila nueva de `gafira-theme`), V12 y ARQ-107 en verde. V14 falla por `InsertAsset` sin prueba de propiedades en `gafira-edit/tests/prop_commands.rs`: viene de la spec 011, ya está en `develop` y esta rama no toca `gafira-edit`.
- Rojo verificado: con `from_json` y `contrast_ratio` sustituidos por versiones vacías fallaban 18 de 22 tests; los 4 restantes cubren el parsing de hex, las claves de error y las fuentes.
- Mutantes a mano: muertos los del mínimo de contraste, el umbral de los pares rechazados, la orientación del par rechazado, el desajuste de modos en los dos sentidos, `motion.reduced`, `schema` 0, el uso exento, el alfa de la sombra y la curva sRGB. Equivalente: detectar el ciclo por longitud de la cadena en vez de por pertenencia (también termina con `ReferenceCycle`). Dos de ellos sobrevivían al principio y llevaron a añadir sus tests.

## Riesgos y decisiones abiertas

- **Fuentes de producto**: el repositorio solo tiene los subconjuntos de prueba `assets/fonts/test/Inter-test.ttf` y `Manrope-test.ttf` (variables, U+0020-007E, U+00A0-017F, U+2010-2022, U+2026, U+20AC) con su entrada en `assets/LICENSES.toml`. `gafira-theme` los embebe de forma provisional. Pendiente del titular: qué archivos de producto (familia completa o subconjunto *latin* + *latin-ext*, variable o estáticas) y dónde viven; al cambiarlos basta con cambiar las dos rutas de `fonts.rs` y registrar los archivos.
- **Sombras del modo oscuro**: 03 §3 dice que el modo oscuro usa `rgba(0,0,0,0.45)` con borde `border`, pero el JSON solo tiene una escala `shadow`. Como los valores no pueden ir en código, el crate expone la escala del JSON tal cual; hace falta que el titular añada la escala oscura al JSON (por ejemplo `shadow.dark`).
- **Lista de tokens obligatorios**: 02 §8.2 paso 3 habla de «cada token que usa `gafira-ui`»; hasta que exista la UI la lista es la de 03 §3 (todos los tokens de `modes.light`), declarada como contrato en `tokens.rs`.
