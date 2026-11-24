# 098 — Gotero, deslizadores del editor y panel Capas

> **Estado:** in-progress — 2026-11-24 — fases 1 y 2 implementadas y probadas en la app (S98-01 a S98-13); pendiente la prueba del titular. Tomar color fuera de la ventana, pendiente de decisión (dependencia de captura).

## Problema y resultado esperado

- No se puede tomar un color de lo que se ve en el lienzo para aplicarlo a otro elemento: cada selector de color solo ofrece el cuadro de egui.
- En Propiedades de un texto, «Posición y tamaño» queda cortado bajo el panel: no hay desplazamiento.
- Los deslizadores del editor mezclan dos estilos: el de «Editar imagen» (spec 085) y el `Slider` de egui.
- Capas (spec 048) no muestra un asa para arrastrar ni un menú por capa.

## Alcance

### Fase 1

- **S98-01 Gotero.** Junto a cada selector de color del editor hay un botón redondo con el icono `eyedropper` (Phosphor 2.1.1, ARQ-75). Al pulsarlo, el siguiente clic en cualquier punto de la ventana toma el color del píxel bajo el puntero (captura de la ventana, `ViewportCommand::Screenshot`), sin seleccionar nada en el lienzo; Esc o clic derecho salen sin color. Se aplica como si se hubiera elegido a mano (un paso de deshacer).
- **S98-02** El color tomado conserva la transparencia del selector; si era totalmente transparente pasa a opaco.
- **S98-03** El color solo llega al selector que armó el gotero, una vez.
- **S98-04 Selector de color.** Muestra redonda (cuadros si es translúcida) que abre: cuadro saturación/brillo, barra de tono, barra de transparencia y fila con la muestra, el código hexadecimal editable (`#RGB` o `#RRGGBB`) y un segundo gotero; tomar un color desde dentro deja el selector abierto.
- **S98-05** Sitios: fondo y patrón de página, relleno y degradados, contorno, colores de grupo, color y efectos de texto, luces, tablas, gráficos, QR, sombras de imagen y Dibujo.
- **S98-06 Deslizador único** (`slider::Slider`): pista fina con la parte hecha en acento, mando redondo, nombre y número editable con unidad del catálogo (ARQ-97), doble clic vuelve al reposo; pasos, decimales, escala logarítmica y forma compacta. Sustituye todos los `egui::Slider` del editor: zoom de la barra inferior (compacta, logarítmica), giro, opacidad, espaciado, interlineado, curva, luces, recorte, tablas, gráficos, QR, borrador, Dibujo y opciones de Exportar.
- **S98-07 Propiedades.** Texto, formas, líneas, grupos, marcos y luces se desplazan cuando no caben; todo elemento muestra su caja como «Editar imagen»: «Posición y tamaño» (X/Y, Ancho/Alto en dos columnas), Giro (deslizador −180°…180°, sin giro para grupos), girar/voltear, «Aspecto» con Opacidad, y Alinear/Orden.

- **S98-01 (lupa).** Al armar el gotero se toma una captura de la ventana; bajo el puntero (cruz) se ve una lupa circular de 11 × 11 píxeles ampliados con rejilla, el píxel central enmarcado y su código; el clic lo toma al momento.

### Fase 2

- **S98-08 Capas.** Filas en tarjeta (40 px) con asa de seis puntos, miniatura por tipo (relleno de la forma, trazo de la línea o icono), nombre en una línea cortado con «…» (los saltos de línea de un texto pasan a espacios), y a la derecha ojo, candado y «⋮»; el ojo y el candado se ven en la fila bajo el puntero o seleccionada, y siempre si está oculta o bloqueada. Arrastrar desde cualquier punto de la fila muestra una copia junto al puntero y una línea de inserción entre hermanas; la lista se desplaza en sus bordes y con la rueda. «⋮» o clic derecho abre el menú: Copiar, Pegar, Duplicar, Eliminar, Capa ▸ (orden), Alinear a la página ▸, Cambiar nombre, Bloquear, Ocultar y, en una imagen, Usar como fondo de la página.
- **S98-09 Glifos.** La fuente de interfaz es un subconjunto (spec 014): los símbolos que no tiene («⏵», «›», «✓», «✔», «⚠», «←», «−») se cambian por iconos Phosphor (`caret-right`, `check`, `warning`, `arrow-left`) o por «–». Un test recorre los catálogos y los literales de la UI y falla si algún carácter no tiene glifo.
- **S98-10 Menús.** Los menús de la barra superior y sus submenús son listas sin relleno en reposo, con resalte al pasar y filas holgadas; Ver muestra «✓» (icono) en lo activo en lugar de botones seleccionados. «Tamaño» y «Notas» se ocultan hasta que funcionen (`SHOW_UNFINISHED`).
- **S98-11 Colores predeterminados.** En texto, forma y fondo de página: muestras circulares en filas de siete, «Colores sólidos predeterminados» (4 filas) y «Colores degradados predeterminados» (3 filas), cada una con «Ver todo», que muestra en el panel la paleta completa (grises y 18 tonos de oscuro a claro, 133 colores) o los degradados por grupos (Tonos fríos, Tonos cálidos, Monocromático, Todos los degradados); «←» vuelve.

- **S98-12 Campo de color único** (`color_field::ColorField`): título, «Sin …» cuando se puede quitar, selector con gotero y «Color personalizado», colores de la marca y del diseño (círculos) y los predeterminados con «Ver todo». Lo usan el color del texto, el relleno y el contorno de una forma (este solo con colores sólidos) y el fondo de la página; con varias formas, sin «Ver todo».
- **S98-13 Comillas.** Sección «Comillas» en Formas con 30 diseños propios: redondas, gruesas, rectas, suaves, inclinadas, cuñas, finas, verticales, angulares y cuadradas (normal, cierre, simple, ancha, estrecha, gancho largo), dobles y simples, de apertura y cierre, e insignias redonda y cuadrada con las comillas caladas. Las plantillas «Frase en grande» y «Frase del día» usan estas formas (`gafira_ui::piece_geometry`) en lugar de la letra «“».

- **S98-14 Paquetes plegables.** En los paneles con «Paquetes instalados» (Ilustraciones, Iconos, Formas…), cada paquete es una cabecera con flecha, miniatura, nombre y número de recursos que se abre y se cierra con un clic; cerrados por defecto, se abren solos al buscar o al elegir un paquete en el filtro.
- **S98-15 Desinstalar desde el panel.** Cada cabecera de paquete tiene un botón «⋮» (y clic derecho) con «Desinstalar paquete»; pide confirmación bajo la cabecera y desinstala con el mismo almacén que Ajustes → Paquetes. Los paneles y Plantillas se vuelven a leer sin reiniciar; si el filtro mostraba ese paquete, vuelve a «Todos los paquetes». Los diseños que usan sus recursos los conservan (spec 097).

## Fuera de alcance

- La pantalla Marca conserva su editor de color propio.
- Tomar color fuera de la ventana de Gafira: exige una dependencia de captura de pantalla (decisión del titular).

## Dependencias

Specs 037 (iconos), 048 (Capas), 063 (degradados), 085 (deslizador de imagen), 088. Sin cambio de formato.

## TDD y evidencia

- `eyedropper::tests` (s98_01 a s98_05): píxel bajo el puntero por escala, fuera de la imagen, transparencia, entrega a un solo selector, captura → color, Esc, código hexadecimal.
- `eyedropper::tests` también: lupa alrededor del píxel y clic con la lupa.
- `layers::tests` (s98_08): nombre en una línea. `presets::tests` (s98_10): HSL, filas de oscuro a claro, degradados sin repetir. `widgets::glyph_tests` (s98_09).
- `slider::tests` (s98_06): escala lineal, pasos desde el inicio del rango, escala logarítmica.
- Prueba manual en la app (2026-11-24): gotero del fondo de página tomando las comillas de «Frase en grande»; panel de texto con desplazamiento y «Posición y tamaño» visible; selector desplegado.

## Riesgos

- La captura incluye lo que se dibuja encima (marcos de selección): se toma lo que se ve.
