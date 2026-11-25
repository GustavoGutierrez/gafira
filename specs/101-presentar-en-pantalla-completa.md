# 101 — Presentar en pantalla completa

> **Estado:** in-progress — 2026-11-25 — implementada y probada en Linux (botón, Ctrl+Alt+P, flechas, transiciones, barra que se oculta, dibujo, puntero láser, Esc); pendiente del titular.

## Problema y resultado esperado

Un diseño de varias páginas (presentaciones, carruseles) no se podía mostrar a pantalla completa. El resultado: un botón en la barra inferior del editor y el atajo **Ctrl+Alt+P** (Cmd+Alt+P en macOS) muestran las páginas una a una a pantalla completa, como una presentación.

## Alcance

- Botón «Presentar en pantalla completa» en la barra inferior, a la derecha de «Páginas n de N», con su atajo en el tooltip.
- La ventana pasa a pantalla completa; cada página se dibuja tan grande como cabe en la pantalla, centrada sobre un fondo oscuro (token `present.bg`), a la resolución real de la pantalla (máx. 4096 px por lado).
- Empieza en la página actual; las páginas ocultas no se presentan.
- Teclado: siguiente con →, ↓, Espacio, Av Pág e Intro; anterior con ←, ↑, Re Pág y Retroceso; Inicio y Fin; Esc sale. Un clic en la diapositiva pasa a la siguiente.
- Barra inferior de la presentación: línea de progreso, anterior, «n/N», siguiente y «Salir de la pantalla completa». Se ve al empezar (2,5 s) y cuando el puntero está cerca del borde inferior; se oculta, con el cursor, cuando el puntero se aleja.
- Al salir la ventana vuelve a su tamaño si no estaba a pantalla completa, y el editor queda en la última página presentada.

- Entrada sin saltos: mientras la ventana crece a pantalla completa solo se ve el fondo; la diapositiva aparece con un fundido cuando el tamaño de la pantalla se mantiene.
- Transición entre páginas: fundido cruzado con un desplazamiento corto hacia el lado al que se avanza.
- Herramientas: puntero láser (L) con estela; dibujo (D) con bolígrafo, rotulador y resaltador en seis colores; borrador (E); borrar los dibujos de la página (C). Los dibujos son de la presentación, no se guardan en el diseño. Esc deja primero la herramienta y después sale.

## Fuera de alcance

Notas del presentador, temporizador, vista en una segunda pantalla, otras transiciones, guardar los dibujos, reproducción de vídeo o animación.

## Requisitos

- **S101-01** Botón y atajo Ctrl+Alt+P abren la presentación; Esc o el botón de salida la cierran.
- **S101-02** Cada página llena la pantalla conservando su proporción, ampliando las pequeñas y sin pasar de 4096 px (`ExportService::present`).
- **S101-03** Navegación página a página dentro de los límites del diseño, empezando por la página actual.
- **S101-04** Teclas de navegación y salida descritas en el alcance.
- **S101-05** La barra inferior se oculta cuando el puntero se aleja del borde inferior y reaparece al acercarse.
- **S101-06** La diapositiva no se muestra hasta que la pantalla tiene su tamaño final; entra con un fundido.
- **S101-07** Cambiar de página tiene una transición de fundido y desplazamiento.
- **S101-08** Puntero láser y dibujo con borrador sobre la diapositiva, en coordenadas de la propia diapositiva.

## Pruebas

- `s101_02_a_page_fits_the_screen` (`gafira-engine`, tamaño de la página presentada).
- `s101_03_it_moves_within_the_pages` (`gafira-ui`, navegación y borrador).
- Revisión en la app (Linux): botón, Ctrl+Alt+P, flecha derecha a la página 2, barra oculta y visible según el puntero, Esc vuelve a la ventana de 1440×900; diseño vertical (1080×1920) proporcional y centrado; trazo de bolígrafo y estela del láser.

## Implementación

`crates/gafira-ui/src/present.rs` (estado, teclado, dibujo y barra), `ExportService::present` en `gafira-engine`, botón y atajo en `editor.rs`. Las páginas se dibujan con el mismo render de las exportaciones, una por fotograma (la actual primero, luego sus vecinas) y se guardan por tamaño de pantalla. Los iconos `caret-left`, `corners-in`, `pen`, `marker-circle`, `highlighter`, `eraser`, `crosshair` y `broom` de Phosphor se añadieron con su entrada en `assets/LICENSES.toml`.
