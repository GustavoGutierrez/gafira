# 080 — Códigos QR

> **Estado:** in-progress — 2026-11-22 — iniciada por orden del titular (100 % sin red, en Rust y eficiente); implementada y probada; el titular probó la interfaz y la dio por buena. Pendiente: leer con teléfonos los marcos no cuadrados, y «Guardar mi estilo» (S80-14). El modelo es `NodeKind::Qr` (D-113).

## Objetivo

Hoy Elementos → Códigos QR dice «Disponible pronto.». Al terminar, la persona elige qué contiene el código (una dirección, una red Wi-Fi, un contacto…), lo personaliza (colores, degradado, formas, logotipo), lo ve al momento y lo añade al lienzo como vectores. Después puede volver a editarlo: el código se regenera sin perder su sitio, tamaño ni estilo. Todo ocurre en el equipo: 100 % sin red, en Rust y sin binarios externos (titular, 2026-11-22).

## Alcance / no alcance

Alcance, en tres fases que se entregan por separado:

1. **Base**: modelo, generador, tipos URL, Texto, Correo, Teléfono, SMS y Wi-Fi; colores lisos, 6 formas de cuerpo y 4 de cada parte del ojo; panel, inserción, edición y exportación.
2. **Personalización**: degradados, colores de los ojos, el catálogo completo de formas, logotipo y estilos predefinidos.
3. **Tipos largos**: Contacto (vCard y MeCard), Evento, Ubicación, Red social y Criptomoneda.

No alcance: códigos dinámicos, estadísticas, archivos alojados y cualquier servicio de red; búsqueda de direcciones en un mapa (se escriben latitud y longitud); galería de logotipos de marcas (son marcas registradas); Micro QR, rMQR y otros códigos de barras; EPS; leer códigos; creación en masa (spec futura).

## Dependencias

- FD-20 y RF-16 (esta spec los implementa y los amplía); D-102 y D-105 (Elementos; clic y arrastre); D-101 (degradados); D-107 (precedente de recurso dentro del nodo); ARQ-04 (sin red); ARQ-11, ARQ-137 y ADR-02 (extensiones); ADR 0006.
- Specs 054 (trazados), 063 (degradados), 064 (Elementos), 070 (Propiedades), 073 y 074 (PDF y SVG desde las operaciones de la página), 075 (PPTX).
- Crate nuevo de dominio `gafira-qr` (capa 2; depende de `gafira-doc`, `gafira-geom`, `gafira-color` y `gafira-time`): fila V5 y copia de `clippy.toml` (V12); `gafira-render`, `gafira-io-pptx` y `gafira-ui` pasan a depender de él (la fila V5 de `gafira-render`, también).
- `qrcode` 0.15.0 (MIT OR Apache-2.0) sin sus funciones por defecto (`image`, `svg`, `pic`, `eps`), solo para la matriz. FD-20 verificó la 0.14.1: repetir `cargo deny check`.
- `rqrr` 0.11 ((MIT OR Apache-2.0) AND ISC), **solo de desarrollo** (en `gafira-testkit`), como decodificador de prueba: cierra FD-20 (a) y el punto de QR del ítem 151 de 07. Su localizador no se usa (exige ojos cuadrados): la prueba muestrea los módulos y exige en los tres ojos la proporción 1:1:3:1:1 que buscan los lectores.

## Diseño

**Modelo** (`gafira-doc`, aditivo en el esquema 1): `NodeKind::Qr(QrNode)`.

| Campo | Contenido |
| --- | --- |
| `content: QrContent` | Enum por tipo con sus campos tal como se escribieron; el texto codificado se calcula, no se guarda. |
| `ecc` | `l`, `m`, `q` o `h`; por defecto `m`. |
| `quiet_zone` | Módulos por lado, 0–10; por defecto 4. |
| `style: QrStyle` | `body`, `eye_frame`, `eye_ball`, `mirror_eyes`; `fill` (color o `Gradient`), `eye_frame_fill` y `eye_ball_fill` opcionales (`None` toma `fill`) y `background` opcional (`None` es transparente). |
| `logo: Option<QrLogo>` | `asset`, `natural`, `share` (0,10–0,30 del lado del código sin margen) y `clear_behind`. |

`Property::Qr(QrNode)` cambia el nodo entero en un paso de deshacer. La caja es cuadrada: si se deforma, el código se centra en el lado menor.

**Generador** (`gafira-qr`), en funciones puras y deterministas: `encode(&QrContent) → texto`, `matrix(texto, ecc) → Matrix` (con qué módulos son de función) y `outlines(&Matrix, &QrStyle, …) → QrDrawing` (cuerpo, marcos, centros, fondo y hueco del logotipo como `BezPath`, en un espacio de 1 unidad por módulo). El grafo de render, el lienzo, las miniaturas y el PPTX usan ese único `QrDrawing`, con caché por contenido y estilo.

**Contornos.** Cada capa es un solo trazado compuesto. Los módulos contiguos se unen: no quedan aristas internas. Las formas «conectadas» miran los 8 vecinos; los ojos nunca se dibujan con la forma del cuerpo.

**Catálogo** (propio y paramétrico):

| Parte | Formas |
| --- | --- |
| Cuerpo suelto | Cuadrado, Cuadrado separado, Punto, Punto pequeño, Rombo, Estrella, Cruz |
| Cuerpo conectado | Fluido, Fluido con rincones, Barras horizontales, Barras verticales, Biselado, Hoja |
| Marco del ojo | Cuadrado, Redondeado, Círculo, Hoja, Gota, Biselado, Punteado, Octógono |
| Centro del ojo | Cuadrado, Redondeado, Círculo, Hoja, Rombo, Barras, Cojín, Flor |

Fase 1: Cuadrado, Cuadrado separado, Punto, Fluido y las dos Barras; en los ojos, Cuadrado, Redondeado, Círculo y Hoja. `mirror_eyes` orienta las formas asimétricas hacia el centro del código.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S80-01 | Local. | Generar, editar y exportar no abre conexiones (ARQ-04); el contenido no se escribe en registros ni en mensajes de error. 300 caracteres se generan en < 50 ms (FD-20). |
| S80-02 | Modelo. | Ida y vuelta JSON exacta de cada tipo; un rango fuera de límites, un color no finito o un logotipo sin recurso no validan; un documento sin QR conserva sus bytes. Editar es un paso de deshacer. |
| S80-03 | Contenido por tipo. | **URL**: exige esquema; sin él se antepone `https://`. **Texto**: UTF-8. **Correo**: `mailto:` con asunto y cuerpo en codificación porcentual. **Teléfono**: `tel:` solo con `+` y cifras. **SMS**: `SMSTO:número:texto`. **Wi-Fi**: `WIFI:T:…;S:…;P:…;H:…;;` con `\ ; , : "` escapados; sin clave, `T:nopass` y sin `P`. **Contacto**: vCard 3.0 con `CRLF` y `\ ; ,` y saltos escapados, o MeCard con sus escapes. **Evento**: `VCALENDAR` con un `VEVENT`, hora local; el fin no precede al inicio. **Ubicación**: `geo:lat,lon` (±90, ±180), punto decimal en cualquier idioma. **Red social**: dirección del perfil o de la publicación. **Criptomoneda**: `moneda:dirección?amount=…`; Ethereum, solo `ethereum:dirección`. |
| S80-04 | Validación. | Campo obligatorio vacío o inválido: aviso junto al campo y «Añadir al diseño» desactivado; en un código ya insertado se conserva el último válido. Si no cabe (2331 bytes en M, 1273 en H): «El contenido no cabe en un código QR. Acórtalo o baja la corrección.» No se baja sola. |
| S80-05 | Vectores. | Cada capa es un trazado; con Cuadrado, su área es exactamente la de los módulos oscuros y no hay aristas internas. Al 3200 % no se ven rendijas ni pixelado. El trazado se rellena igual con la regla par-impar y con la de no cero. |
| S80-06 | Formas. | Cuerpo, marco y centro se eligen por separado en baldosas dibujadas por el generador. Todas las combinaciones de las tres listas, en negro sobre blanco, las lee el lector de prueba. |
| S80-07 | Colores. | Color de frente liso o degradado (lineal con ángulo, o radial) de los de la spec 063, extendido sobre todo el código; «Intercambiar» invierte sus extremos. Marco y centro del ojo con color propio o «como el frente». Fondo liso o transparente. Muestras de la Marca. |
| S80-08 | Contraste. | Con el peor par entre frente (cada parada), ojos y fondo por debajo de 3:1: «Este código puede no leerse bien.» (FD-20). Con el frente más claro que el fondo: «Algunos lectores no leen códigos claros sobre fondo oscuro.» Con fondo transparente, el aviso dice que depende de lo que haya detrás. Avisan, no bloquean. |
| S80-09 | Corrección y margen. | L, M, Q y H, con su recuperación aproximada (7, 15, 25 y 30 %); margen 0–10, 4 por defecto; con menos de 4: «Con menos margen, algunos lectores no encuentran el código.» |
| S80-10 | Logotipo. | Imagen PNG o JPEG importada, centrada, con «Quitar los módulos de detrás». Ponerlo sube la corrección a H; mientras esté, no se puede bajar; quitarlo devuelve la anterior. Nunca tapa ojos, sincronía ni formato: `share` se recorta a lo que cabe. Con el tamaño máximo permitido, el lector de prueba lo lee en las versiones 2 a 40; en la 1 se usa la 2. |
| S80-11 | Panel e inserción. | Elementos → Códigos QR: vista previa fija arriba, lista «Tipo», y secciones Contenido, Colores, Formas y Logotipo. La vista previa cambia al escribir (< 50 ms). «Añadir al diseño» lo pone en el centro y arrastrar la vista previa lo suelta centrado en el puntero (D-105): cuadrado, 25 % del lado corto de la página, seleccionado, un paso de deshacer. El panel recuerda el último estilo, no el último contenido. |
| S80-12 | Edición y exportación. | Con un QR seleccionado, Propiedades muestra los mismos controles; una ráfaga de escritura en un campo es un paso de deshacer. Las esquinas conservan la proporción. Se ve igual en el lienzo, PNG, JPEG, WebP, PDF (vectores) y SVG (vectores, sin referencias externas); en PPTX sale como formas. Un SVG y un PDF exportados, dibujados y leídos, devuelven el contenido. |
| S80-13 | Convertir en formas. | «Convertir en formas» lo cambia por un grupo de `ShapeNode` (fondo, cuerpo, marcos, centros) y, si hay, la imagen, sin cambio visible, en un paso de deshacer; desde ahí ya no se regenera. |
| S80-14 | Estilos. | Ocho estilos propios, ninguno con avisos de contraste. **Pendiente**: «Guardar mi estilo» en los ajustes locales. |
| S80-15 | Accesibilidad e idioma. | Textos en español e inglés (D-08) y en el glosario de 03; cada baldosa con nombre accesible y foco visible; los avisos no dependen solo del color; campos con `widgets::field` (DIS-88). |
| S80-16 | Privacidad. | El nombre en Capas es «Código QR», nunca el contenido. Con Wi-Fi, el panel avisa: «La clave se guarda en el proyecto y cualquiera que lea el código puede verla.» |
| S80-17 | Avisos en vivo (titular, 2026-11-22). | Cada campo obligatorio lleva un asterisco; un ejemplo en gris dice qué se espera. Un valor mal escrito (dirección web, correo, teléfono, clave WPA, fecha, coordenadas, importe) se avisa mientras se escribe, bajo su campo, con el borde de error, una marca y cómo corregirlo; un obligatorio vacío, en cuanto el campo se ha usado. Se avisan todos los campos a la vez y bajo «Añadir al diseño» se dice qué falta o «Listo para añadir.», sin que el formulario salte. |
| S80-18 | Alineación (titular, 2026-11-22). | La etiqueta va sobre su campo y cada campo ocupa el ancho de la columna o una de dos mitades iguales y a la misma altura; en las filas con etiqueta a la izquierda, los controles se alinean en el borde derecho. Nada queda bajo la barra de desplazamiento. |

## TDD

Casos que deben fallar primero:

1. `s80_02_qr_round_trip` y `s80_02_qr_validation` (cada límite, por dentro y por fuera).
2. `s80_02_the_qr_is_changed_and_undone`.
3. `s80_03_wifi_escapes_the_reserved_characters`: SSID `Cafe;Uno`, clave `a:b\c,d"e`.
4. `s80_03_mailto_percent_encodes`: asunto `Hola & adiós`.
5. `s80_03_vcard_escapes_and_uses_crlf`: nombre `Ana;María`, calle `Calle 1, 2`.
6. `s80_03_event_is_a_calendar`, `s80_03_geo_limits`, `s80_03_mecard_escapes`.
7. `s80_04_capacity_per_level`: el último byte que cabe y el siguiente.
8. `s80_05_square_area_is_the_dark_modules` (propiedad, versiones 1–40) y `s80_05_no_inner_edges`.
9. `s80_05_both_fill_rules_agree`, para cada forma.
10. `s80_06_every_style_reads_back` (el lector de prueba sobre el dibujo a 8 px por módulo; 13 × 8 × 8 combinaciones, y cada cuerpo en un código pequeño, uno mediano y uno grande).
11. `s80_08_worst_pair_contrast` (3:1 justo, por debajo, degradado e invertido).
12. `s80_10_logo_never_covers_function_modules` y `s80_10_the_largest_logo_reads_back`.
13. `s80_10_a_logo_raises_and_restores_the_level`.
14. `s80_17_*` (direcciones, correos, longitudes, todos los problemas, cuándo se avisa) y `s80_18_the_fields_line_up`.

Integración por función: `gafira-render` (operaciones del QR), `gafira-io-pdf` y `gafira-io-svg` (exportar, dibujar y leer), `gafira-io-pptx`, `gafira-ui` (`d105_a_dropped_qr_lands_where_it_was_dropped`, `s80_13_converting_keeps_the_picture`) y `s80_01_a_bench_of_300_characters`.

## Evidencia

- Tests: `gafira-doc/tests/qr.rs`, `gafira-edit` (`s80_02_*`), `gafira-qr/tests` (contenido, formas y lectura de las 832 combinaciones), `gafira-render-cpu/tests/qr.rs`, `gafira-io-svg` y `gafira-io-pptx` (`s80_12_*`), `gafira-ui` (`qr.rs`, `editor.rs`, `export.rs`).
- Exportación real desde el motor con formas, degradado y logotipo: PNG, JPEG, WebP y SVG leídos de vuelta; el PDF (dibujado con poppler, con el logotipo como única imagen) y el PPTX (dibujado con LibreOffice), también.
- `cargo deny check licenses bans sources`: correcto con `qrcode` 0.15.0 y `rqrr` 0.11.0; los avisos de `advisories` son anteriores a esta spec.
- Suite completa: 3441 tests, 0 fallos (2026-11-22); fmt, clippy, V5, V12, V13 y ARQ-107 correctos. V14 falla desde antes (`DuplicatePage` y `ReorderPage` sin test de propiedades).
- Interfaz probada en la app y por el titular, que además leyó con su teléfono un código con todo cuadrado.
- El motor ZXing (`rxing`, usado una vez fuera del repo) leyó los PNG, JPEG, WebP, PDF y PPTX exportados con marcos Hoja, centros Flor (antes de ensancharla), degradado y logotipo.

## Evidencia esperada

Tests anteriores en verde; gates de `AGENTS.md` (incluidos `cargo deny check`, V5 y V12); un código de cada tipo leído con dos teléfonos (Android e iOS), con el contenido y el resultado anotados; un PNG, un PDF y un SVG exportados y leídos; prueba del titular en la app. Al cerrar, actualizar FD-20 en 04, el glosario de 03, el ítem 151 de 07 y `README.md`.

## Riesgos y decisiones abiertas

**A. Modelo (cerrada, D-113).** FD-20 y 04 pedían un nodo `Extension` con `type_id = "gafira.qr"`. El titular ratificó `NodeKind::Qr`, que sustituye esa línea y FD-20 (c).

| | `NodeKind::Qr` (recomendado) | `Extension` «gafira.qr» (FD-20) |
| --- | --- | --- |
| Logotipo | El documento valida el recurso y la exportación lo decodifica, como en los marcos. | El `payload` es opaco: ni la validación ni la exportación ven el recurso. |
| Trabajo previo | Ninguno: mismo camino que líneas y marcos. | Crear el registro de ARQ-137 (no existe: hoy una extensión no se dibuja), su propiedad editable y su dibujo en el lienzo y el PPTX. |
| Versión anterior de Gafira | No abre el archivo (como con marcos, líneas y luces). | Lo abre y conserva el nodo sin dibujarlo. |

**B. Cantidad de formas.** El catálogo propone 13, 8 y 8. Ampliarlo después no cambia el formato.

**Riesgos**

- El lector de OpenCV (que busca esquinas rectas) no lee los marcos de ojo que no son cuadrados; el motor ZXing lee todos. El titular leyó con teléfono el cuadrado; faltan los demás marcos en Android e iOS.
- Los lectores difieren: el lector de prueba comprueba la estructura, no la cámara. Por eso la evidencia pide dos teléfonos.
- Formas con poca tinta (Punto pequeño, Estrella, Cruz): si una no se lee, se engorda o se retira del catálogo, no se baja la prueba.
- Texto con acentos en modo byte sin ECI: los lectores actuales lo detectan como UTF-8; si un teléfono falla, se añade ECI 26.
- La red de una página impresa pequeña: un módulo de menos de 0,4 mm se lee mal. Aviso de tamaño físico, para una spec posterior.
- «QR Code» es marca registrada de DENSO WAVE: la interfaz dice «Códigos QR»; «Acerca de» incluye el reconocimiento.
