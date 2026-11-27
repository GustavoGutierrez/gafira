# 108 — Redimensionar el diseño

> **Estado:** in-progress — 2026-11-27 — pedida por el titular con capturas de referencia; en implementación.

## Objetivo

Que el diseño abierto cambie de tamaño desde la barra superior, en todas sus páginas o en la actual, y que su contenido se escale y se recoloque para encajar en el nuevo tamaño, en el mismo diseño o en una copia. Cumple la entrada «Cambiar tamaño» del documento 03 (§2, §7.6: «Adaptar este diseño» y «Duplicar y adaptar»).

## Alcance / no alcance

Alcance:

- Botón «Redimensionar» en la barra superior del editor (sustituye al «Tamaño» oculto de la spec 098). Abre un panel desplegable anclado al botón.
- Vista principal del panel:
  - un buscador de tamaños («Buscar opciones para redimensionar diseños»);
  - «Sugerencias»: tres tamaños comunes del catálogo de la spec 076;
  - «Busca por categoría»: «Elegir el tamaño» y las categorías del catálogo (redes sociales, presentaciones, videos, imprimir, documentos, fotos, web), cada una con su lista de tamaños.
- Vista «Elegir el tamaño», que reemplaza el contenido del panel y tiene «‹» para volver: casilla, ancho, alto, unidades (px, mm, cm, in) y candado de proporción. Al abrirla trae el tamaño de la página actual.
- Un único tamaño elegido a la vez, marcado con su casilla; elegir otro desmarca el anterior.
- Pie fijo:
  - alcance «Todas las páginas (N)» o «Esta página», en un desplegable;
  - «Copiar y redimensionar»: abre una copia nueva sin guardar del diseño, ya redimensionada; si el diseño tiene cambios sin guardar, primero se pregunta como al salir;
  - «Redimensionar el diseño»: cambia el diseño abierto en un solo paso de deshacer.
  - Los dos botones están desactivados mientras no haya un tamaño válido elegido.
- Encaje del contenido: escala uniforme, la mayor que deja entero lo que cabía en la página (`min(W/w, H/h)`), centrado. Letras, contornos, radios de esquina, grosor de líneas y texto dentro de formas se escalan con la misma regla que las plantillas (spec 091). Las guías se recolocan con la misma escala y desplazamiento. Márgenes, sangrado, fondo y cuadrícula se conservan.

No alcance:

- Varios tamaños a la vez (varias casillas que crean varias copias).
- «Rediseño Mágico» y reacomodo inteligente por tipo de elemento.
- «Ver todo» de las sugerencias como pantalla aparte; los tableros virtuales y «Sitio web» como formatos especiales.

## Dependencias

- Catálogo de tamaños y búsqueda (spec 076, `design_sizes.rs`), encaje de plantillas (spec 091, `template_fit.rs`), `PageSize` y sus límites (`gafira-doc`), historial y `Command` (`gafira-edit`), copia del proyecto (`ProjectService::save_copy`, `new_document_from_template`), diálogo «¿Guardar los cambios?» del shell.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S108-01 | Botón «Redimensionar» en la barra superior | Visible en el editor; un clic abre el panel y un clic fuera lo cierra |
| S108-02 | Búsqueda, sugerencias y categorías | Escribir filtra los tamaños del catálogo por nombre y palabras; cada categoría abre su lista con «‹» para volver |
| S108-03 | «Elegir el tamaño» | Ancho, alto y unidades editables; el candado conserva la proporción; los valores fuera de los límites de `PageSize` dejan los botones desactivados |
| S108-04 | Encaje del contenido | Cada elemento se escala por `s = min(W/w, H/h)` y se desplaza para quedar centrado; letras, contornos, radios y líneas se escalan por `s`; las guías se recolocan; los ids no cambian |
| S108-05 | «Redimensionar el diseño» | Cambia el tamaño de las páginas del alcance y su contenido en un solo paso de deshacer; deshacer devuelve el diseño exacto |
| S108-06 | Alcance | «Todas las páginas» cambia todas; «Esta página» solo la actual |
| S108-07 | «Copiar y redimensionar» | Abre una copia nueva sin guardar ya redimensionada; el original queda intacto en su archivo; con cambios sin guardar pregunta primero |

## Pruebas (TDD en la lógica crítica)

- `gafira-edit`: `ResizePages` sustituye las páginas por id, conserva el orden y el resto del documento, rechaza una página que no existe o un tamaño fuera de límites, y `revert` devuelve el documento exacto (S108-05).
- `gafira-ui`: el encaje de una página (S108-04) con un rectángulo, un texto y una guía: escala, centrado, letras, contorno y guía; página sin tamaño válido devuelve `None`.
- Integración en el editor: «Redimensionar el diseño» con alcance todas o una página, un paso de deshacer (S108-05, S108-06).

## Evidencia para cerrar

- Pruebas `s108_` en verde (`git grep s108_`), gates de `AGENTS.md` y prueba del titular en la app.

## Riesgos y decisiones abiertas

- Encaje por escala uniforme y centrado: en cambios de proporción grandes (cuadrado a historia vertical) queda espacio vacío arriba y abajo. El reacomodo inteligente queda fuera de esta spec.
- «Copiar y redimensionar» reemplaza el diseño abierto en la ventana (Gafira no tiene pestañas); el original se conserva en su archivo.
