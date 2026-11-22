# 087 — Importar WebP y AVIF, exportar AVIF

> **Estado:** in-progress — 2026-11-22 — implementada y probada en Linux; pendiente de la prueba del titular. Pedida por el titular.

## Objetivo

Traer imágenes `.jpg`, `.jpeg`, `.webp` y `.avif` por todas las entradas (arrastrar, pegar, Biblioteca, fondo, marcos, logotipo), y exportar en AVIF.

## Alcance / no alcance

Alcance: AVIF de 8 a 12 bits, con opacidad, en mosaico (*grid*), girado o reflejado; exportar con calidad y fondo transparente.

No alcance: animaciones AVIF y WebP (se usa el primer fotograma cuando existe como imagen), perfiles ICC y HDR (se leen como sRGB), EXIF al exportar.

## Dependencias

D-118 (esta spec), FD-55, specs 032 y 039. Crates nuevos: `re_rav1d` (BSD-2-Clause) y `ravif` (BSD-3-Clause).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S87-01 | Contenedor. | Lector propio de AVIF; un archivo roto o cortado da error, nunca un fallo, y no reserva memoria sin límite. |
| S87-02 | Decodificar. | 4:0:0, 4:2:0 y 4:4:4, rango completo y limitado, opacidad, mosaico, giro; frente a una referencia, diferencia media ≤ 3/255. |
| S87-03 | Importar. | Las extensiones nuevas entran por todas las puertas y se ven, se editan y se exportan como un PNG. |
| S87-04 | Exportar. | «AVIF» en el diálogo: escala, Baja, Media y Alta, fondo transparente; se vuelve a leer con los mismos píxeles dentro de la pérdida. |

## TDD

`gafira-io-image` (contenedor, decodificación, exportador) y `gafira-ui` (entradas y diálogo).

## Evidencia

- Gates de 086 a 089 juntas: `cargo fmt --check`, `clippy -D warnings`, `cargo deny` y `cargo test --workspace --locked` en verde, 3636 tests. Fallan desde antes V14 y `forbid(unsafe_code)` en `gafira-photos`.

- Medido en release (i7-12700H), 1920 × 1080: escribir AVIF 0,8 s; leerlo 0,05 s.
- Los archivos de prueba los escribió otro programa (libavif); el mosaico lo arma el test. `cargo deny check licenses bans sources`: en verde.
- Probado en la app: pegar un `.avif` y verlo. Con tests, sin probar a mano: exportar AVIF desde el diálogo, arrastrar WebP. No probado: AVIF de cámaras, Windows ni macOS.
- Tests: `s87_*` en `gafira-io-image`.

## Riesgos y decisiones abiertas

- Codificar AVIF en CPU es más lento que PNG o JPEG; se hace en otro hilo, como toda exportación.
