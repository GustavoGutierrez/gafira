# 027 — Texto en el lienzo

> **Estado:** done — 2026-11-16 — S27-01..S27-04 cubiertos por `gafira-doc` (`text::tests` 2, `validate::tests` 1, contratos ARQ-134 ampliados), `gafira-ui` (`canvas::tests` 1, `editor::tests` 1); 2813 tests.

## Objetivo

Añadir y editar texto en el Editor de diseño (03 §2.3 Texto, §7.3): el elemento de texto entra en el modelo del documento (02 §3.1, `TextNode`), se dibuja en el lienzo y se edita desde el Inspector con deshacer.

## Alcance / no alcance

Alcance:

- `gafira-doc`: primer corte de `TextNode` (02 §3.1, ADR 0008): `content: RichText` (párrafos de tramos con `SpanStyle { font, size, color }`), `sizing: TextSizing` y `align: TextAlign`; `NodeKind::Text`; validación del tamaño (`0 < size ≤ 4096`) y del color de cada tramo (`NodeField::FontSize`, `TextColorChannel`, `TextColorAlpha`); `NodeVisitor::visit_text`. Campo nuevo de `schemaVersion` 1 (ADR 0006).
- `gafira-edit`: `Property::TextContent(RichText)` con la misma validación; presupuesto de memoria del texto.
- `gafira-ui`: pestaña Texto con «Añadir un título», «Añadir un subtítulo» y «Añadir texto» (familia de títulos o de interfaz del tema, color tinta de la marca, palabras iniciales del catálogo); el texto se dibuja ajustado al ancho de su caja y alineado; se selecciona, mueve, redimensiona y ordena como cualquier elemento; el Inspector edita las palabras, el tamaño y el color.

No alcance:

- Edición directa en el lienzo (doble clic), tramos con estilos distintos, negrita y cursiva, columnas, «Ajustar al cuadro», «Fijar tipografía», interletrado, interlineado, texto en curva, caja que crece con el texto.
- Dibujar con `gafira-text`: el lienzo usa la maquetación de egui hasta que pinte a través de `gafira-render`.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S27-01 | El texto del documento. | Una línea por párrafo; el texto plano vuelve igual; forma de cable con etiquetas y valores por defecto al leer. |
| S27-02 | Validación. | Tamaño 0, mayor que 4096 o no finito, o alfa fuera de [0, 1], se rechazan con el campo correcto. |
| S27-03 | En el lienzo como una caja. | Caja centrada del 60 % del ancho y una línea de alto; se lista como «Texto» en Capas y recibe los clics. |
| S27-04 | Por el engine. | Insertar un título usa la familia de títulos y queda seleccionado; editar y cambiar el tamaño llegan al documento; un tamaño 0 se rechaza; un deshacer devuelve el título. |
