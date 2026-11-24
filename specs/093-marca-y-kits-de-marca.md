# 093 — Marca: kits de marca y plantillas de marca

> **Estado:** in-progress — 2026-11-23 — implementada y probada en Linux (D-124); pendiente de la prueba del titular.

## Objetivo

Una persona guarda la identidad de una o varias marcas (kits de marca) y la usa al diseñar: logos, colores, estilos de texto, fuentes subidas, fotos, gráficos e iconos a mano en el panel Marca del editor y en los selectores de color. Convierte un diseño en una plantilla de marca, o sube una, y crea piezas nuevas desde ellas.

## Alcance / no alcance

Alcance:

- **Kits de marca**: varios por persona, cada uno con nombre; se crean con solo el nombre (modal «Crear kit de marca») y se completan después. Guardan logos (PNG, JPEG, WebP, AVIF, SVG), paletas con nombre (cada color en HEX y, si se quiere, su CMYK para impresión), estilos de texto con nombre (Título, Subtítulo, Encabezado de sección y Cuerpo por defecto; se añaden y se borran; fuente, tamaño, negrita, cursiva), fuentes subidas (TTF, OTF), fotos, elementos gráficos e iconos.
- **Pantalla Marca** (barra superior, junto a Inicio y Plantillas): todos los kits como tarjetas (logo, tira de colores, nombre y «···» con Abrir, Cambiar el nombre y Eliminar) y «Añadir uno nuevo»; la página de un kit con sus secciones (Logos, Colores, Fuentes tipográficas, Fotos, Elementos gráficos, Iconos y Plantillas de la marca), cada una con «Añadir uno nuevo» y su estado vacío; la lista de la izquierda lleva a cada sección. Los colores se eligen en un selector flotante (cuadro, tono, HEX y CMYK) o se extraen de una imagen (del kit o del equipo): Gafira detecta sus colores principales y la persona elige cuáles van a la paleta; los estilos se editan en su fila con un selector de fuente con búsqueda (subidas primero, y «Subir fuente» en la misma lista) y un desplegable de tamaños. «Fuentes subidas», siempre a la vista, muestra cada familia en su propia letra con su número de archivos, sube varias a la vez y las desinstala. «Plantillas de la marca» reúne las de todos los kits.
- **Panel Marca del editor** (pestaña del riel, hasta ahora apagada): plantillas de la marca, selector de kit con «Gestionar», logos, colores, estilos de texto, fotos, gráficos e iconos. Se distinguen las acciones: un recurso se inserta al pulsarlo o se arrastra a la página; un estilo de texto se aplica a los textos seleccionados sin cambiar lo escrito, o, sin selección, añade un texto; un color se aplica a los textos seleccionados, si no a sus formas, si no al fondo de la página; una plantilla va a la página.
- **Colores de la marca en los selectores de color** del editor.
- **Plantillas de marca**: «Guardar como plantilla de marca» desde Archivo convierte el diseño abierto en una plantilla del kit elegido; «Subir plantilla» copia un `.gaf` y lo marca como plantilla. Se usan como las plantillas incluidas: abrir como diseño nuevo o aplicar en el editor.

No alcance:

- Equipos, permisos, controles de marca, borradores y publicación: Gafira es local y de una persona.
- Crear un kit desde una web o un PDF; videos; temas de color y «Mezclar»; voz de la marca (el titular la quitó).
- Que cambiar un color del kit cambie los diseños ya hechos.

## Diseño

- Los kits viven fuera de los documentos, en la carpeta de datos (`brand/kits.json` y `brand/<kit>/` con sus archivos y plantillas), como Biblioteca. La interfaz habla con ellos por el puerto `BrandStore`; la aplicación lo implementa.
- Una plantilla de marca es un `.gaf` con el `template` de D-81 y la etiqueta `brand:<kit>`. `gafira-format::set_template` copia un `.gaf` poniendo o cambiando su `template`.
- Insertar un logo, una foto o un gráfico usa el camino de importación de Biblioteca; un texto con estilo, el de la pestaña Texto.
- Las fuentes subidas son los archivos de las carpetas `fonts/` de los kits: de cada familia se toma, como en el escaneo del sistema, el peso más cercano al normal (400) y a la negrita (700), en recta y cursiva (`gafira_platform::faces_of_files`), y se ponen en el catálogo (`SystemFonts::set_added`) aparte de lo que escanea; el lienzo, la exportación y todas las listas de fuentes las encuentran por el mismo camino. Desinstalar borra los archivos de la familia y vuelve a calcular el catálogo. Cualquier parte de Gafira que dibuja una familia del sistema o subida la carga la primera vez (`system_fonts::load_family`).
- Los diálogos del sistema (Abrir, Guardar, Importar y los de Marca) se abren con la ventana de Gafira como padre, para que el gestor de ventanas los ponga delante.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S93-01 | `set_template`. | Copia un `.gaf` con el `template` dado; el resto de entradas, igual; la copia se abre y se lista como plantilla. |
| S93-02 | Kits. | Crear, renombrar y eliminar; se guardan y se leen igual; un kit nuevo solo necesita nombre. |
| S93-03 | Recursos. | Añadir y quitar logos, fotos, gráficos, iconos y fuentes (se copian a la carpeta del kit; quitar borra la copia; una fuente subida está en todo Gafira y al desinstalarla sale), colores en HEX y CMYK con nombre de paleta, y estilos de texto. |
| S93-04 | Pantalla Marca. | Selector de kit, «Todos los recursos» y secciones; estados vacíos con su acción. |
| S93-05 | Panel del editor. | Insertar logo, foto, gráfico o texto con estilo; aplicar un color a la selección; abrir la gestión. |
| S93-06 | Colores en los selectores. | Los colores del kit activo aparecen en los selectores de color del editor. |
| S93-08 | Colores de una imagen. | Los colores principales de una imagen (PNG, JPEG, WebP, AVIF o SVG), el de mayor área primero, sin sus partes transparentes; la persona elige cuáles se añaden y no se repiten los que la paleta ya tiene. |
| S93-07 | Plantillas de marca. | Guardar el diseño como plantilla de marca, subir un `.gaf`, listarlas por kit y usarlas como diseño nuevo o en la página. |

## TDD

1. `gafira-format`: `set_template` conserva entradas y pone el `template`.
2. Kits: ida y vuelta de `kits.json`, ids únicos, nombre por defecto.
3. Plantillas de marca: filtro por kit.
4. Colores de una imagen: orden por área, transparencia ignorada, SVG, archivo que no es imagen.

## Evidencia

- `gafira-format`: `s93_01_set_template_marks_a_project_and_keeps_the_rest`.
- `gafira-ui`: `brand::tests` (colores de una imagen PNG y SVG, y de lo que no es imagen; kit nuevo e id libre, colores de todas las paletas, CMYK a pantalla, HEX, etiquetas de plantilla de marca), `brand_screen::tests` (CMYK escrito como cuatro números), `system_fonts` (fuentes añadidas una vez y encontradas).
- `apps/gafira`: `brand::tests` (kits guardados y leídos igual, el activo, borrar con sus archivos; archivos copiados sin pisarse; una fuente subida se instala y se desinstala).
- `gafira-platform`: `s93_03_uploaded_files_keep_the_weight_nearest_each_slot`.
- Xephyr: crear «Gynaika» desde el modal; su página con colores (selector flotante), estilos (edición en la fila, lista de fuentes flotante, tamaños); en un diseño, el panel Marca: «Título» añade un texto con su estilo, un color se aplica al texto seleccionado, «Subtítulo» cambia el estilo sin tocar lo escrito, «Colores de la marca» en el Inspector; Archivo > Guardar como plantilla de marca la añade al kit y al panel; «Extraer de una imagen» sobre el logo SVG encuentra 5 colores y añade los 3 elegidos; los logos y las fotos se ven sin deformarse (4:3 y 1:3) y el botón de borrar se queda quieto al pasar por encima; con Black Ops One, IBM Plex Mono (14 archivos) y Poppins (18): «Fuentes subidas (3)», cada familia en su letra y en su peso normal, «Cuerpo» en Poppins negrita, y desinstalar IBM Plex Mono borra sus 14 archivos.

## Riesgos y decisiones abiertas

- 03 §7.7 imaginaba la identidad dentro del documento (`VisualIdentity`, `SetSwatch`); esta spec la guarda fuera, como Biblioteca, y deja para después que un cambio del kit actualice los diseños.
