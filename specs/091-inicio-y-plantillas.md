# 091 — Inicio nuevo y Plantillas

> **Estado:** in-progress — 2026-11-23 — implementada y probada en Linux (D-122); pendiente de la prueba del titular.

## Objetivo

Inicio pasa a ser una portada: un título, dos pestañas (Inicio, Plantillas), un buscador, accesos directos a las categorías de «Crear un diseño», los proyectos recientes y plantillas agrupadas por tipo. «Crear un diseño» (spec 076) se conserva y se abre desde esos accesos, ya en la categoría pulsada. Plantillas es una pantalla propia para explorar todas las plantillas con búsqueda, filtros y carga progresiva; una plantilla abre un diseño nuevo con su contenido, editable por completo.

## Alcance / no alcance

Alcance:

- Inicio nuevo (03 §7.2, §6.2 «Elegir»): título «¿Qué vas a diseñar hoy?», pestañas Inicio y Plantillas, buscador, fila de accesos, «Recientes» y «Plantillas para ti» por tipo.
- El buscador de Inicio filtra los tipos de diseño (los tamaños de la spec 076) y las plantillas: «Facebook» muestra los tamaños de Facebook y las plantillas de esos tamaños.
- Accesos directos: Redes sociales, Presentaciones, Video, Imprimir, Documentos, Fotos, Web y correo, Elegir el tamaño, Plantillas y Abrir. Cada uno abre «Crear un diseño» en su categoría, Plantillas abre la pantalla de plantillas y Abrir el selector de archivos.
- Pantalla Plantillas: buscador, chips de categoría, «Explorar por tipo» (tarjetas de tipo) y la cuadrícula de todas, que carga por páginas al desplazarse.
- Plantilla: un `.gaf` con el campo `template` de D-81. Abrirla crea un documento nuevo, sin ruta ni versiones, con un `projectId` nuevo; nada de la plantilla queda vinculado.
- Catálogo de plantillas incluidas en la aplicación, ampliable dejando un `.gaf` en `assets/templates/`.
- Un primer juego de plantillas propias (redes, presentación, impresos, documentos, video).
- Barra superior: Plantillas activa y lleva a la pantalla de plantillas.
- Pestaña Plantillas del riel del editor: busca entre las plantillas de proporción parecida a la página activa; una plantilla reemplaza el contenido y el fondo de esa página, ajustada a su tamaño, y sus demás páginas se añaden detrás, en un solo paso que se deshace de una vez.

No alcance:

- «Guardar como plantilla» y las plantillas propias de la persona (03 §2.3); el catálogo ya lee cualquier `.gaf` con `template`, así que se añadirán después sin cambiar el formato.
- Plantillas descargadas de la red.
- Proyectos (la pestaña de la barra sigue apagada).

## Dependencias

- D-81 (campo `template` del manifiesto), spec 076 (Crear un diseño, tamaños), spec 033 (miniaturas en el `.gaf`), spec 019/031 (lectura y escritura del `.gaf`).
- Guía de marca (`gafira-brand`): tokens, Manrope en el título, campos con `widgets::field`.

## Diseño

- `gafira-format` lee y escribe `template = { tags, pageFormat, style, color }` (D-81) y ofrece `read_template_info`, que lee solo el manifiesto y la miniatura, como `read_thumbnail`.
- `gafira-engine`: `ProjectService::new_document_from_template(path)` carga el `.gaf`, le da un `projectId` nuevo, sin ruta, sin versiones y sin modificar; la plantilla no se bloquea ni se toca.
- El tipo de una plantilla es el primer tag que nombra un tamaño de la spec 076 (`size.ig_square`); de él salen su categoría, su nombre de tipo y las palabras de búsqueda. Un tag `lang:es` o `lang:en` la marca de un idioma; sin él es neutra. Se muestran las del idioma de la interfaz y las neutras (01 §7.3).
- Las plantillas van en `assets/templates/<categoría>/`, con nombres de archivo en inglés y el idioma antes de la extensión (`corner-coffee.es.gaf`). Su nombre visible es la etiqueta `title:`; sin ella, el nombre del archivo. Las carpetas son también palabras de búsqueda.
- La búsqueda vale en los dos idiomas: cada plantilla lleva su nombre en el otro idioma como etiqueta, y los nombres de tipo y categoría se comparan en español y en inglés; los tipos de diseño de Inicio también.
- Las plantillas incluidas las genera `apps/gafira/examples/make_templates.rs` (`cargo run -p gafira --example make_templates`) con el render de la exportación; cualquier `.gaf` con `template` que se deje en la carpeta se lista sin tocar código.
- Cada diseño puede llevar su paleta (café, bayas, océano, bosque, violeta, monocromo, atardecer, carmesí): el generador cambia los colores de la marca por los de la paleta según su papel.
- Los espacios de foto son marcos vacíos (spec 066) con una muestra debajo, porque un marco vacío no se imprime; al soltar una foto en el marco, la tapa.
- Cada página lleva las guías de su tamaño en «Crear un diseño» (`gafira_ui::size_guides`): pliegues bloqueados del tríptico y márgenes de seguridad. Las de impresión llevan 3 mm de sangría, y las formas que tocan un borde llegan hasta ella.
- `ApplyTemplate` da a las páginas que añade las guías, los márgenes y la sangría de la página activa.
- Las plantillas incluidas van dentro del binario (no dependen de la instalación). El catálogo se lee una vez; las miniaturas se decodifican en un hilo aparte y se muestran a medida que llegan.
- Carga progresiva: la cuadrícula muestra páginas de 24 y añade la siguiente al acercarse al final.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S91-01 | Campo `template`. | Un `.gaf` escrito con `template` lo devuelve igual al leerlo; uno sin él es un proyecto normal; un `template` mal formado no impide abrir el proyecto y se ignora. |
| S91-02 | Información de plantilla sin abrirla. | `read_template_info` devuelve el `template` y la miniatura de la primera página sin leer páginas ni recursos; un proyecto sin `template` no es plantilla. |
| S91-03 | Diseño desde plantilla. | El documento nuevo tiene el contenido de la plantilla, otro `projectId`, sin ruta ni versiones, sin cambios pendientes; deshacer no lo vacía; la plantilla no cambia. |
| S91-04 | Catálogo. | Lista las plantillas incluidas con id, nombre, tipo, categoría y miniatura; filtra por categoría, por tipo, por idioma y por texto; las páginas de 24 cubren todas sin repetir. |
| S91-05 | Búsqueda de Inicio. | Una búsqueda muestra los tipos de diseño que coinciden (spec 076) y las plantillas que coinciden; un tipo abre el editor con su tamaño; una plantilla, con su contenido. |
| S91-06 | Accesos. | Cada acceso abre «Crear un diseño» en su categoría (Elegir el tamaño incluido), Plantillas abre la pantalla de plantillas y Abrir el selector. «Crear un diseño» vuelve a Inicio. |
| S91-07 | Inicio. | Muestra título, pestañas, buscador, accesos, «Recientes» y «Plantillas para ti» agrupadas por tipo con «Ver todo» que abre Plantillas filtrada. |
| S91-08 | Pantalla Plantillas. | Buscador, chips de categoría, tipos, cuadrícula con carga progresiva y estado vacío; un clic en una plantilla abre el editor con ella. |
| S91-09 | Primer juego de plantillas. | Al menos 20 plantillas, de al menos cinco categorías, que cargan sin error, con miniatura, en español y en inglés. |
| S91-13 | Detalle de plantilla. | Un clic en una plantilla de Inicio o de Plantillas abre un modal con su imagen, nombre, tipo, tamaño y categoría, «Personalizar esta plantilla» y plantillas similares (mismo tipo, luego misma categoría, del idioma de la interfaz); una similar se muestra en el mismo modal; Esc, la cruz o un clic fuera lo cierran. |
| S91-14 | Fotos, guías y sangría. | Los espacios de foto son marcos; las plantillas llevan las guías de su tamaño; las de impresión, 3 mm de sangría; las páginas que añade `ApplyTemplate` heredan guías, márgenes y sangría. |
| S91-12 | Ampliable. | Añadir un `.gaf` con `template` a `assets/templates/` lo muestra en el catálogo sin tocar código. |
| S91-10 | Aplicar en el editor. | `ApplyTemplate` pone la primera página de la plantilla en la página activa (que conserva id, nombre, tamaño y guías) y añade las demás detrás, con ids nuevos y las líneas pegadas a sus formas; un deshacer lo quita todo; cumple las leyes de los comandos. `read_template` lee la plantilla sin tocar la sesión. |
| S91-11 | Proporción y encaje. | El panel solo ofrece plantillas a menos de un 12 % de la proporción de la página; la plantilla se escala entera y centrada, con letras, contornos y esquinas a escala. |

## TDD

Casos que deben fallar primero:

1. `gafira-format`: ida y vuelta de `template`; un `template` mal formado se ignora; `read_template_info` sin páginas leídas.
2. `gafira-engine`: `new_document_from_template` con `projectId` nuevo, sin ruta ni versiones ni cambios; la plantilla intacta.
3. `gafira-ui`: filtro del catálogo por categoría, tipo, idioma y texto; páginas sin repetir; tipo a partir de los tags.
4. Las plantillas incluidas cargan, tienen tipo conocido y miniatura (prueba de integración).

## Evidencia esperada

- Pruebas nuevas en verde, suite completa, `fmt`, `clippy` y política.
- Capturas de Inicio, Plantillas y un diseño abierto desde una plantilla en Xephyr.

## Evidencia

- `gafira-format`: `tests/templates.rs` (S91-01, S91-02): ida y vuelta del `template`, proyecto sin él, plantilla que abre, cinco `template` mal formados ignorados.
- `gafira-engine`: `s91_03_a_template_opens_as_a_new_untitled_document`: otro `projectId`, sin ruta ni versiones ni cambios, guardable aunque la plantilla cargue de solo lectura, archivo intacto y sin bloqueo.
- `gafira-ui`: `templates::tests` (S91-04, S91-07): nombre por `title:`, tipo, categoría e idioma, filtros, búsqueda en el otro idioma, páginas sin repetir y filas por tipo.
- `gafira-edit`: `tests/apply_template.rs` (S91-10) y `apply_template_obeys_the_command_laws` (V14); `gafira-engine`: `s91_10_a_template_is_read_without_touching_the_session`; `gafira-ui`: `template_fit::tests` (S91-11).
- Xephyr: en un post cuadrado el panel ofrece solo las dos plantillas cuadradas y «Café de barrio» llena la página, que pasa a «Usado recién»; en una presentación 16:9 ofrece las apaisadas parecidas y «Proyecto creativo» deja tres páginas; un deshacer vuelve a una página vacía.
- `templates::tests::s91_13_similar_templates_share_the_kind_then_the_category`; Xephyr: el modal de «Café de barrio» muestra sus similares, una similar cambia el modal y «Personalizar esta plantilla» abre el diseño.
- `apps/gafira/tests/templates.rs` (S91-09): 120 plantillas (60 diseños × 2 idiomas: 26 redes sociales, 10 invitaciones, 10 presentaciones, 6 trípticos, 4 impresos, 1 documento, 2 web y 1 video) en 7 carpetas abren, con tipo, nombre, idioma y miniatura, y nombres de archivo en inglés. Todas suman unos 2,8 MB.
- Xephyr con datos temporales: Inicio con accesos, recientes y plantillas por tipo; «coffee» con la interfaz en español encuentra «Café de barrio»; «facebook» muestra los tres tipos de Facebook y su plantilla; una plantilla abre «Sin título» editable; un acceso abre «Crear un diseño» en su categoría, con «Volver a Inicio» y «Plantillas»; la pestaña Plantillas con chips, tipos y cuadrícula.

## Riesgos y decisiones abiertas

- El tamaño del binario crece con las plantillas incluidas; se vigila (objetivo: menos de 5 MB entre todas).
- Las plantillas iniciales solo usan formas, textos y los colores de la marca: sin fotos ni ilustraciones de terceros.
- «Usado recién» del panel dura lo que la sesión.
