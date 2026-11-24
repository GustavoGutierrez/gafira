# 098 — Gotero, deslizadores del editor y panel Capas

> **Estado:** in-progress — 2026-11-24 — fase 1 (gotero, selector de color, deslizadores, Propiedades de texto) implementada y probada en la app; fase 2 (panel Capas) en curso.

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

### Fase 2

- **S98-08 Capas.** Filas con asa de arrastre a la izquierda, línea de inserción al arrastrar, ocultar y bloquear a la derecha y un menú «⋮» por capa (copiar, pegar, duplicar, eliminar, orden, alinear a la página, bloquear, ocultar); desplazamiento cuando hay muchas capas.

## Fuera de alcance

- La pantalla Marca conserva su editor de color propio.
- Tomar color fuera de la ventana de Gafira.

## Dependencias

Specs 037 (iconos), 048 (Capas), 063 (degradados), 085 (deslizador de imagen), 088. Sin cambio de formato.

## TDD y evidencia

- `eyedropper::tests` (s98_01 a s98_05): píxel bajo el puntero por escala, fuera de la imagen, transparencia, entrega a un solo selector, captura → color, Esc, código hexadecimal.
- `slider::tests` (s98_06): escala lineal, pasos desde el inicio del rango, escala logarítmica.
- Prueba manual en la app (2026-11-24): gotero del fondo de página tomando las comillas de «Frase en grande»; panel de texto con desplazamiento y «Posición y tamaño» visible; selector desplegado.

## Riesgos

- La captura incluye lo que se dibuja encima (marcos de selección): se toma lo que se ve.
