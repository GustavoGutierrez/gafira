# 084 — Mejorar la resolución, quitar el fondo y borrar píxeles

> **Estado:** in-progress — 2026-11-22 — implementada y probada en Linux; pendiente de la prueba del titular. Pedida por el titular con capturas de referencia.

## Objetivo

Al seleccionar una imagen, Propiedades ofrece «Mejorar resolución» y «Quitar fondo», con modelos locales ligeros por defecto y otros que se pueden descargar según la máquina; con el fondo quitado se pueden borrar y restaurar píxeles. Nada destruye la imagen original.

## Alcance / no alcance

Alcance: los elementos de imagen; modelos integrados y descargables; CPU en Rust puro y GPU opcional; efecto de escaneo en el menta de la marca mientras se procesa; borrador de píxeles; restaurar el original.

No alcance: fotos dentro de marcos, vídeo y lotes; modelos de la persona (el catálogo es datos, pero no hay interfaz para añadir uno); `Aislar sujeto` clásico (FD-38), que sigue como estaba; bloquear la función sin licencia (D-12, D-62).

## Dependencias

FD-38, FD-39, FD-40; D-115 (esta spec), D-111 (precedente de red); ARQ-08, ARQ-76, RNF-03, RNF-04; 02 §9.2 (`ort` solo con `load-dynamic`). Crate adaptador nuevo `gafira-vision`. Dependencias: `tract-onnx` (MIT/Apache-2.0) y `ort` (MIT/Apache-2.0, opcional); `cargo deny` en verde.

## Modelos

| Id | Tarea | Peso | Licencia | Procedencia comprobada | Entrega |
| --- | --- | --- | --- | --- | --- |
| `u2netp` | quitar fondo | 4,6 MB | Apache-2.0 | U²-Net; publicación de rembg, md5 de su lista | integrado |
| `u2net` | quitar fondo | 176 MB | Apache-2.0 | igual | descarga |
| `u2net-human` | quitar fondo, personas | 176 MB | Apache-2.0 | igual | descarga |
| `isnet-general` | quitar fondo, detalle | 179 MB | Apache-2.0 | IS-Net (DIS); publicación de rembg | descarga |
| `realesr-general-x4v3` | mejorar | 4,9 MB | BSD-3-Clause | exportado aquí del `.pth` oficial (`tools/export_realesr.py`) | integrado |
| `realesrgan-x4plus` | mejorar | 67 MB | BSD-3-Clause | ONNX de terceros cuya salida iguala la del `.pth` oficial (diferencia máxima 2·10⁻⁶) | descarga |

**ClearReality V1, propuesto por el titular, no se usa**: su autor lo publica como CC-BY-NC-SA-4.0 (ficha de OpenModelDB) y Gafira es de pago; el «MIT» del repositorio que lo resube no lo cambia. Otros descartados: `bria-rmbg` (no comercial), BiRefNet (falla en GPU y 5 s en CPU), `real_esrgan_x4` de otro repositorio (su salida no coincide con la oficial), SPAN ×2 (procedencia sin comprobar).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S84-01 | No destructivo. | La imagen guarda su original y su base sin recortar; «Restaurar original» vuelve a ella aunque se haya guardado y cerrado. Cada operación es un paso de deshacer y conserva la caja. |
| S84-02 | Calidad. | Antes de quitar el fondo se mide: pequeña (menos de 800 px), borrosa o con bloques de compresión. Solo si es mala se mejora antes; si la mejora falla, se quita el fondo igual. |
| S84-03 | Polimorfismo. | `BackgroundRemover` y `Upscaler` en `gafira-ports`; un modelo es datos (tamaño, normalización, huella, licencia, fuente) y un motor es un `Runtime`: añadir uno no toca la interfaz ni el motor de edición. |
| S84-04 | Recursos. | `tract` en CPU siempre; ONNX Runtime (GPU por WebGPU, o su CPU) si la persona lo instala. Para cada modelo se mide una vez cada motor y se usa el más rápido; si uno falla se usa el siguiente. |
| S84-05 | Modelos. | Un panel muestra la máquina, los modelos con peso, licencia, velocidad medida, cuál es el recomendado y por qué; descargar, usar y borrar. Nada se descarga sin que la persona lo pida. |
| S84-06 | Descarga. | HTTPS, tamaño y huella BLAKE3 fijos, archivo temporal y cambio atómico; se cancela; un archivo que no coincide se borra y no se usa. |
| S84-07 | Interfaz. | No se bloquea: progreso, cancelar y efecto de escaneo en menta (acento de la marca; el titular descartó el púrpura) sobre la imagen (con movimiento reducido, sin barrido). Los errores dicen qué pasó y qué hacer. |
| S84-08 | Borrador. | «Borrar píxeles»: Borrar y Restaurar, tamaño 1–500, comparar, deshacer por trazo; Listo aplica la sesión como un paso y Cancelar la descarta. |
| S84-09 | Sin red. | Editar y exportar funcionan sin red con los modelos integrados. |
| S84-10 | Presupuesto. | Quitar fondo de una imagen de 4000 px en menos de 5 s en CPU (CAL-06) y 2 s con GPU (CAL-07). |

## TDD

`gafira-doc/tests/image_edit.rs`, `gafira-edit`, `gafira-raster` (calidad), `gafira-engine` (flujo), `gafira-vision` (tensores, teselas, catálogo, descarga con transporte falso, elección de motor) y `gafira-ui` (sesión de borrado).

## Evidencia

- `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` y `cargo test --workspace --locked`: en verde, 3574 tests. V5, V12, V13, ARQ-06, ARQ-07 y ARQ-107, en verde; fallan desde antes V14 (`DuplicatePage`, `ReorderPage`) y `forbid(unsafe_code)` en `gafira-photos`. `cargo deny check licenses bans sources`: en verde.
- Medido en release (i7-12700H, RTX 4070 portátil), quitar fondo: 512 px, 0,36 s con `u2netp` y 0,65 s con `u2net` en CPU (`tract`); 0,03 y 0,1 s con GPU. 4000 × 4000 px: 1,4 s en CPU y 1,1 s con GPU (S84-10, CAL-06 y CAL-07). Mejorar 512 → 2048 px: 2,9 s con `realesr-general-x4v3` en CPU; 4 s con `realesrgan-x4plus` en GPU. Preparar un modelo la primera vez cuesta de 0,8 a 6 s, y más de 20 s con `x4plus` sin GPU.
- Los umbrales de calidad se calibraron con nueve fotos nítidas, desenfocadas (1 a 5 px) y comprimidas (calidad 50 a 5).
- El `.onnx` integrado se reproduce byte a byte con `tools/export_realesr.py`.
- Probado en la app (Linux, datos temporales): quitar fondo con mejora previa, efecto, borrador (borrar, restaurar, deshacer, Listo), restaurar original, panel de modelos, descarga real de 271 MB y proceso por GPU. No probado: Windows y macOS.
- La prueba en la app encontró y corrigió que Ctrl+Z en el borrador deshacía el documento (`s84_08_undo_is_of_the_strokes`).
- Corrección pedida por el titular (2026-11-22): los botones Listo y Cancelar del borrador tienen margen, y la ventana muestra una barra de estado con el tamaño, la posición del puntero y el control de zoom (`s84_08_the_zoom`).

## Riesgos y decisiones abiertas

- La GPU en Windows y macOS usa la misma biblioteca de ONNX Runtime, pero no se ha probado en esas máquinas.
