# 112 — Firma de paquetes `.gafpack`

> **Estado:** in-progress — 2026-11-28 — firma Ed25519, verificación al inspeccionar e instalar, autor verificado en la ventana del paquete y en Ajustes → Paquetes; `gafpack.py` genera claves y firma.

## Problema y resultado esperado

Hoy Gafira muestra en todos los paquetes «autor declarado, no verificado» (097 §9.3): los hashes del manifiesto detectan cambios en los recursos, pero no dicen quién publicó el paquete. Los paquetes oficiales de Gafira se ven igual que los de cualquiera.

Resultado: un paquete firmado con una clave de publicador que Gafira conoce se muestra como **autor verificado** («Gafira · autor verificado») en la ventana de instalación y en Ajustes → Paquetes. Un paquete con la firma rota o con contenido cambiado después de firmar se rechaza. Los paquetes sin firma siguen instalándose como hasta ahora, con autor declarado y no verificado.

## Alcance

- Archivo de firma `META-INF/signature.json` dentro del `.gafpack` (097 §3 reserva `META-INF/` para esto).
- Algoritmo Ed25519 sobre un mensaje canónico que cubre el SHA-256 de **todos** los archivos del paquete fuera de `META-INF/` (manifiesto, miniatura, recursos, previsualizaciones y licencia).
- Lista de claves de publicadores confiables incluida en Gafira (`TRUSTED_KEYS`); la primera es la de Gafira (`gafira-2026`).
- Verificación al inspeccionar y al instalar; el estado se guarda en `install.json` al instalar.
- Textos ES/EN de autor verificado y de los nuevos problemas.
- `scripts/gafpack.py`: `keygen`, `sign`, `build --sign-key` y `check` que informa del estado de la firma.

## Fuera de alcance

- Revocación, caducidad o rotación automática de claves; un almacén de publicadores editable por el usuario; catálogo remoto.
- Firmar con certificados X.509 o servicios externos.
- Volver a verificar los paquetes ya instalados cada vez que se listan (se confía en `install.json`, escrito por Gafira al instalar).

## Dependencias y supuestos

- Spec 097 (formato 1.1, `PackStore`, `install.json`).
- `ed25519-dalek` 3 (BSD-3-Clause, permitido por `deny.toml`) en `gafira-ui`; `sha2` ya está.
- La clave privada vive fuera del repositorio, en el equipo del publicador; nunca se versiona. En el repositorio solo está la clave pública.

## Formato

`META-INF/signature.json`:

```json
{
  "format": "gafpack-signature",
  "version": 1,
  "algorithm": "ed25519",
  "keyId": "gafira-2026",
  "publisher": "Gafira",
  "files": { "manifest.json": "<sha256 hex>", "thumb.webp": "<sha256 hex>", "…": "…" },
  "signature": "<128 hex>"
}
```

Mensaje firmado (UTF-8, `\n` al final de cada línea, archivos ordenados por ruta en bytes):

```
gafpack-signature/1
key:<keyId>
publisher:<publisher>
<sha256>  <ruta>
…
```

## Requisitos

- **FIR-01** Un paquete sin `META-INF/signature.json` es *sin firma*: se instala como antes y se muestra como autor declarado, no verificado.
- **FIR-02** Si la firma no se puede leer, su `format`, `version` o `algorithm` no son los esperados, o la firma no verifica con la clave confiable de su `keyId`, el paquete se rechaza (`packs.bad_signature`).
- **FIR-03** Si el mapa `files` no coincide exactamente con los archivos del paquete fuera de `META-INF/` (falta uno, sobra uno o cambia un hash), el paquete se rechaza (`packs.signature_mismatch`).
- **FIR-04** Si la firma verifica pero `publisher` no es el publicador de esa clave, se rechaza (`packs.bad_signature`); si `author.name` del manifiesto no es ese publicador, se rechaza (`packs.signature_author`).
- **FIR-05** Una firma con un `keyId` que Gafira no conoce y con `files` coherente deja el paquete como *firmado por una clave desconocida*: se instala y se muestra como no verificado.
- **FIR-06** Un paquete con firma válida de una clave confiable se muestra como «{publicador} · autor verificado» en la ventana del paquete y en Ajustes → Paquetes, y la nota bajo el estado dice que Gafira comprobó la firma y que el contenido no cambió.
- **FIR-07** `gafpack.py` genera un par de claves, firma un paquete existente o al construirlo, y `check` dice si la firma es válida, de qué publicador y si es una clave confiable; produce firmas que Gafira acepta.

## Criterios de aceptación

- [ ] Las reglas FIR-01 a FIR-05 están en una función pura de `gafira-ui` con tests.
- [ ] Un vector de prueba firmado con Python se verifica en Rust (mismo mensaje canónico).
- [ ] `apps/gafira`: un paquete firmado se inspecciona e instala como verificado; uno alterado tras firmar se rechaza; uno sin firma se instala como antes.
- [ ] La ventana del paquete y Ajustes → Paquetes muestran el autor verificado.
- [ ] `gafpack.py sign` + `check` sobre un paquete real lo dan por válido y verificado.
- [ ] Puertas de `AGENTS.md` en verde.

## Casos TDD (fallan primero)

1. `fir_01_no_signature_is_unsigned`.
2. `fir_02_a_bad_signature_is_refused` (firma cambiada, algoritmo desconocido, JSON roto).
3. `fir_03_files_must_match_the_package` (falta, sobra, hash distinto).
4. `fir_04_publisher_and_author_must_match_the_key`.
5. `fir_05_an_unknown_key_is_not_verified`.
6. `fir_06_a_trusted_signature_verifies` y `fir_07_python_vector_verifies`.

## Evidencia esperada

Comandos de las puertas y su resultado; salida de `gafpack.py check` de un paquete firmado.

## Riesgos y decisiones

- Perder la clave privada impide firmar nuevas versiones con la misma identidad; se recomienda copia de seguridad cifrada. Una clave nueva exige una versión de Gafira que la incluya.
- Verificar cuesta leer el paquete una vez más para calcular los hashes; solo se hace cuando hay firma.
- `install.json` lo escribe Gafira; alguien con acceso al disco podría editarlo, pero ese acceso ya permite cambiar los recursos instalados.

## Evidencia

Pendiente.
