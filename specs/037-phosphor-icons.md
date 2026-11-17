# 037 — Iconos Phosphor y botones de icono del editor de diseño

> **Estado:** in-progress — 2026-11-17 — iconos, botones de icono, menú Edición y zoom en curso.

## Objetivo

El editor de diseño deja de usar texto donde 03 pide iconos. La colección es Phosphor (MIT, D-87): peso «Regular» y «Fill» para los estados activos. Los SVG se convierten en geometría al compilar y se dibujan con el color de un token, nítidos a cualquier escala (03 §13). La barra inferior gana los controles de zoom de 03 §7.3 y los atajos de 03 §9.2.

## Alcance / no alcance

Alcance:

- Iconos (`gafira-ui`):
  - `assets/icons/phosphor/{regular,fill}/` con solo los iconos que usa esta spec (19 «Regular» y 6 «Fill»), sin modificar, de `@phosphor-icons/core` 2.1.1. Licencia en `assets/icons/phosphor/LICENSE`.
  - Cada archivo tiene su entrada en `assets/LICENSES.toml` (ARQ-75) y el aviso va en `THIRD_PARTY_NOTICES.md`.
  - `build.rs` lee los SVG y genera una tabla de contornos planos en el binario. Las curvas y los arcos se aproximan con segmentos a 0,5 unidades de 256.
  - `icons.rs` rasteriza cada icono una vez por tamaño en píxeles físicos (cobertura con antialias, regla non-zero) en una textura blanca con alfa. El pintor la tiñe con el token del sitio.
- Componentes (`widgets.rs`):
  - «Botón de icono» (03 §8.1): sin fondo en reposo, icono de 20 px en una zona de 32 px (16 en 28 en la variante pequeña). Hover, pressed, disabled y foco con los tokens de su superficie. Tooltip y nombre accesible salen del catálogo.
  - «Pestaña del riel»: icono de 24 px y etiqueta. La activa usa «Fill», `topbar.fg` y el indicador coral.
  - Botón primario y secundario con icono y etiqueta.
- Editor de diseño:
  - Barra superior: Inicio, Deshacer y Rehacer como botones de icono, con el atajo en el tooltip («Deshacer · Ctrl+Z»). Versiones lleva icono y etiqueta, y Exportar es el botón coral con icono y etiqueta.
  - Menú Edición: Deshacer, Rehacer y Eliminar, sobre las acciones que ya existían.
  - Riel: icono y etiqueta en cada pestaña.
  - Piezas: Rectángulo y Elipse con icono y etiqueta.
  - Capas: Subir, Bajar y Eliminar como botones de icono en la fila seleccionada.
  - Barra inferior: Alejar, Acercar y el menú del zoom (Ajustar, 50 %, 100 %, 200 %).
  - Atajos: Ctrl+= (o Ctrl++) acerca, Ctrl+- aleja, Ctrl+0 ajusta y Ctrl+1 pone el 100 %. El zoom va del 5 al 3200 %.
  - El zoom de toda la interfaz de egui con el teclado queda apagado, porque esos atajos son del lienzo.

No alcance:

- el resto de los ≈ 110 iconos de 03 §13 y `xtask check-icons` (DIS-50 completo);
- el isotipo en el botón Inicio (03 §2.2). Sobre la barra grafito haría falta la variante del símbolo para fondo oscuro, que 03 §5.2 da como pendiente, así que se usa el icono «casa» de Phosphor;
- el slider del zoom de 03 §7.3 y el campo numérico editable (el menú cubre Ajustar y los tres valores fijos);
- el tooltip «Deshacer {acción}» con la etiqueta del comando (02 §4.3);
- la barra contextual, la paleta de comandos y la reasignación de atajos.

## Decisión de implementación

Se usa un parser propio en tiempo de compilación y un rasterizador propio en tiempo de ejecución. No se añade ninguna dependencia nueva al binario; `blake3`, que ya está en el workspace, entra solo como dependencia de desarrollo.

- Al compilar: `build.rs` incluye el mismo módulo `src/icons/path_data.rs` que prueban los tests unitarios. Lee el SVG (solo `<path>` sobre `viewBox="0 0 256 256"`, con todos los comandos `M L H V C S Q T A Z`) y escribe las coordenadas en centésimas de unidad como `i16`. Así el archivo generado no lleva literales `f32`, y un archivo fuera de la rejilla o con otro elemento rompe la compilación.
- Al dibujar: egui no rellena polígonos cóncavos ni con huecos (los contornos de Phosphor los tienen). Una malla sin antialias se vería dentada a 20 px. Por eso se rasteriza por acumulación de área con signo (como font-rs) al tamaño físico exacto. El resultado es una textura cacheada en la memoria de egui por icono, peso y tamaño, que el pintor tiñe.
- Se descartaron:
  - `egui_extras` con `svg` (resvg), porque analiza el SVG en tiempo de ejecución y 03 §13 lo prohíbe;
  - una fuente de iconos, que D-87 excluye;
  - `lyon` o `earcut`, porque serían una dependencia nueva y no dan antialias.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S37-01 | Datos de trazado completos. | El parser lee comandos absolutos y relativos, el lineto implícito, números compactos (`-.5.5`) y banderas de arco pegadas (`011`). Los arcos quedan sobre su círculo y terminan en su punto final, y C/S y Q/T reflejan el control. Lo mal formado es un error, nunca una suposición. Un SVG que no es de 256 unidades o que dibuja con otro elemento se rechaza. |
| S37-02 | Solo lo que se usa, todo dentro de la rejilla. | Cada `Icon` tiene geometría en cada peso, con polígonos de 3 puntos o más dentro de 0..256. Los archivos vendidos son exactamente los que nombra la UI, y la tabla embebida es la lectura de esos archivos. |
| S37-03 | Dibujo nítido y fiel. | El cuadrado «Regular» sale hueco con el trazo lleno. Un borde a 0,75 px cubre el 75 % del píxel. «Fill» sin archivo propio cae en «Regular». |
| S37-04 | Licencia registrada (ARQ-75). | Cada icono vendido tiene entrada MIT, versión 2.1.1, archivo de licencia existente y hash BLAKE3 igual al del archivo. `THIRD_PARTY_NOTICES.md` nombra Phosphor, la versión y el copyright. |
| S37-05 | Nombre accesible (03 §8.1). | El botón de icono es un `Button` con el texto del catálogo como nombre, no el tooltip, y deshabilitado conserva el nombre. La pestaña del riel dice si es la activa. |
| S37-06 | Zoom acotado. | Acercar y Alejar van a la parada siguiente sin repetir la actual y se detienen en 5 % y 3200 %. Ningún valor sale de ese rango, incluidos NaN e infinito. «Ajustar» sigue a la ventana y un valor fijo no. |
| S37-07 | Atajos de 03 §9.2. | Ctrl+1, Ctrl+=, Ctrl+- y Ctrl+0 cambian el zoom del lienzo en el editor real (un frame de egui sin ventana). Sin Ctrl no hacen nada. |

## TDD

Lógica crítica (geometría y límites): S37-01, S37-03 y S37-06. Sus tests se escribieron junto con cada módulo, en la misma unidad de trabajo, no estrictamente antes. Hubo un rojo real en S37-06: un `wheel` extremo daba infinito y `clamp` lo devolvía al 100 %. Se corrigió con el test en rojo: ahora el infinito va a su límite y solo NaN vuelve al 100 %.

## Evidencia

- Tests nuevos en `gafira-ui`:
  - `icons::path_data::tests` (S37-01): 7.
  - `icons::tests` (S37-02, S37-03, S37-04): 8.
  - `widgets::tests` (S37-05): 2.
  - `zoom::tests` (S37-06, S37-07): 4.
  - `editor::tests::s37_07_*`: 1.
- Comprobación visual: el render ASCII a 32 px de «house», «shapes-fill» y «clock-counter-clockwise» sale con los huecos y la orientación correctos.
- Gates: pendientes del cierre.

## Riesgos y decisiones abiertas

- El isotipo del botón Inicio espera la variante del símbolo para fondo oscuro (03 §5.2).
- 03 §13 propone trazo de 1,75 px a 24 px, pero Phosphor «Regular» es de 1,5 px (D-87). Prevalece D-87.
- El color de los iconos deshabilitados sobre la barra grafito es `topbar.fg.muted` al 50 %, porque los tokens no tienen un `topbar.fg.disabled`. Queda propuesto como token para 03 §3.
