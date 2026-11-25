# 103 — Biblioteca de iconos en Elementos → Iconos

> **Estado:** in-progress — 2026-11-25 — implementada y probada en Linux (Tabler integrado por categorías, línea y relleno, búsqueda, insertar y arrastrar); los paquetes de Fluent Emoji quedan para una entrega siguiente.

## Problema y resultado esperado

Elementos → Iconos solo mostraba los iconos de paquetes instalados: sin paquetes estaba vacío. El resultado: Gafira incluye el conjunto Tabler (MIT), unos 5.100 iconos en estilos Línea y Relleno, agrupados por categoría y con búsqueda.

## Alcance

- Conjunto Tabler 3.53.0 sin la categoría de marcas (logotipos registrados), en `assets/icons/tabler/tabler.json.gz` (≈0,4 MB comprimido, ≈2,1 MB en memoria), generado con `tools/gen_tabler_icons.py` y registrado en `assets/LICENSES.toml` con su `LICENSE`.
- Pestañas Línea y Relleno; 40 categorías con nombre en español e inglés (Flechas, Formas, Símbolos, Comunicación, Comercio…); cada categoría muestra sus 10 iconos básicos y «Ver todo».
- Búsqueda por nombre, etiquetas (inglés) y nombre de la categoría en los dos idiomas.
- Un clic añade el icono al centro de la página (un cuarto de su lado menor), un arrastre lo deja donde se suelta. Entra como grupo de formas vectoriales color tinta, con los trazos convertidos en contornos rellenos (`svg_drawing_outlined`), así escalan y se recolorean como cualquier forma.
- Los iconos de paquetes instalados siguen disponibles en un apartado plegable «De tus paquetes».

## Fuera de alcance

Paquetes de Fluent Emoji (planos y 3D) como `.gafpack`; sinónimos en español para la búsqueda por etiquetas.

## Requisitos

- **S103-01** El conjunto se carga con 40 categorías, sin marcas y con los dos estilos.
- **S103-02** La búsqueda encuentra por nombre, etiquetas y categoría en cualquiera de los dos idiomas.
- **S103-03** Insertar y arrastrar un icono crea formas vectoriales con el aspecto del icono.

## Pruebas

- `s103_01_the_set_loads`, `s103_02_the_search_finds_icons` (`gafira-ui`).
- Revisión en la app (Linux): categorías, Relleno, búsqueda «comida», corazón insertado con su contorno y hueco.
