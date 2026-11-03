# Especificación 06: licencia y activación de Gafira

**Estado: ESQUELETO.** Se completa al final del desarrollo, después del MVP técnico de diseño y video. Las secciones marcadas `POR COMPLETAR` no tienen contenido definitivo. Lo ya decidido proviene de `00-decisiones.md` (D-01) y de la arquitectura (`02-arquitectura-software.md`, §4.14, §13 y ADR-08).

Alcance fuera de las primeras fases: esta spec **no bloquea** el desarrollo de las specs 01 a 05.

## 1. Objetivo y alcance

- Permitir vender Gafira como software propietario de pago con prueba gratuita, con una sola licencia que desbloquea todo el producto.
- Verificar la licencia sin conexión después de activarla una vez.
- **Fuera de alcance**: construir un servidor de licencias nuevo. Se reutiliza el `license-hub` ya desplegado del titular (Cloudflare Workers + D1).

`POR COMPLETAR`: criterios de éxito medibles (porcentaje de activaciones sin intervención, tiempo de activación, tolerancia a fallos de red).

## 2. Decisiones ya tomadas (no reabrir)

| ID | Decisión | Origen |
|---|---|---|
| L-01 | Licencia propietaria de código cerrado. Contrato basado en el de Tracewright (`LICENSE` en inglés y `LICENCIA.md` en español, prevalece el español), con tres adaptaciones: nombre, cláusula «Sus resultados» (diseños, imágenes y videos) y permiso para usar plantillas, fuentes y recursos incluidos dentro de las creaciones del usuario. | D-01 |
| L-02 | Todo de pago, con prueba gratuita. Una sola licencia; sin módulos de pago por separado. | D-01 |
| L-03 | Se reutiliza la crate `tracer-license` como base de `gafira-license` y el `license-hub` ya desplegado. | D-01, 02 §4.14 |
| L-04 | La única conexión de red de la app es activar, validar o desactivar la licencia. Sin telemetría. | D-01 |
| L-05 | Verificación sin conexión con token Ed25519 y clave pública fija en el ejecutable. | 02 §4.14 |

## 3. Modelo de estados

Estados de `LicenseStatus` definidos en 02: `Trial`, `Active`, `OfflineGrace`, `Expired`, `Unlicensed`, `Invalid`.

`POR COMPLETAR`:
- Diagrama de transiciones entre estados y qué evento causa cada una.
- Duración y límites de la prueba (Q-19), y si es local desde el primer arranque o con token del `license-hub`.
- Periodo de gracia sin conexión (propuesta de 02: 30 días, validación como mucho una vez al día durante los últimos 7 días).
- Comportamiento sin licencia válida (Q-20; propuesta de 02: modo de solo lectura con abrir, ver, recuperar y guardar copia, sin editar ni exportar).

## 4. Activación y dispositivos

- Identificador de aplicación propuesto: `gafira`.
- El `license-hub` permite de 1 a 10 dispositivos por licencia y activación, validación, desactivación y transferencia en Windows, Linux y macOS (según su README).

`POR COMPLETAR`:
- Flujo de activación paso a paso (clave, dispositivo, respuesta, almacenamiento del secreto).
- Número de dispositivos por licencia de Gafira.
- Transferencia entre equipos y recuperación de un dispositivo perdido.
- Registrar la aplicación `gafira` en el `license-hub` (producto, proveedor de compra, cupos).
- Almacenamiento del secreto del dispositivo por plataforma (llavero del sistema, feature `credentials`).

## 5. Compra y emisión de claves

`POR COMPLETAR`: proveedor de compra (el `license-hub` admite Gumroad o emisión propia), precio, moneda, impuestos, reembolsos, página de compra y correo de entrega.

## 6. Interfaz

`POR COMPLETAR` (se diseña en coordinación con la spec 03, §2 y el sistema de tokens):
- Pantalla de activación y de prueba, con microcopy de marca.
- Estados visibles de licencia en Ajustes (sección Licencia).
- Mensajes de error (sin conexión, clave inválida, límite de dispositivos).
- Modo de solo lectura (Q-20).
- Accesibilidad: teclado completo, foco visible.

## 7. Privacidad

- La activación envía un identificador de dispositivo. Por eso el `PRIVACY.md` de Gafira no puede decir que no se transmiten datos.

`POR COMPLETAR`: lista exacta de campos enviados, retención en el servidor, derechos del usuario y texto final de `PRIVACY.md`.

## 8. Contrato legal

`POR COMPLETAR`:
- Adaptación de `LICENSE` y `LICENCIA.md` según L-01 (tarea del titular).
- Avisos de terceros (`THIRD_PARTY_NOTICES.md`).
- Cláusulas de prueba gratuita y de cancelación.
- Titular, ley aplicable y jurisdicción.

## 9. Seguridad

- La lista de comprobaciones de seguridad del `license-hub` se completa fuera de esta spec (tarea del titular). Su README está desactualizado respecto al estado de despliegue.

`POR COMPLETAR`: modelo de amenazas (clave compartida, manipulación del reloj, parche del ejecutable, extracción de la clave pública), límites aceptados y rotación de la clave de firma.

## 10. Pruebas y criterios de aceptación

`POR COMPLETAR`: matriz de pruebas por estado, simulación de red caída, reloj manipulado, token caducado, clave revocada, y criterios de aceptación numerados (IDs `LIC-xx`).

## 11. Dependencias con otras specs

| Spec | Qué necesita de esta |
|---|---|
| 01 | Fase final «Licencia y distribución comercial» |
| 02 | `gafira-license`, `LicenseVerifier`, `LicenseService`, eventos `LicenseChanged` |
| 03 | Pantalla de activación y sección Licencia en Ajustes |
| 05 | Empaquetado, avisos de terceros y lista previa a vender |

## 12. Preguntas abiertas

| ID | Pregunta | Estado |
|---|---|---|
| Q-19 | ¿La prueba gratuita es local desde el primer arranque o requiere token del `license-hub`? | Abierta |
| Q-20 | ¿Qué permite el modo sin licencia válida? | Abierta (hay propuesta en 02) |
| Q-21 | Precio, número de dispositivos y proveedor de compra | Abierta |
| Q-22 | Contenido final del contrato y de `PRIVACY.md` | Tarea del titular |
