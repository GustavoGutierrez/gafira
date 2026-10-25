# Música generada con MiniMax-Music3

Los WAV de esta carpeta los generó el titular en local con el modelo MiniMax-Music3. Se distribuyen bajo una **excepción** a la regla de assets permisivos de Gafira (decisión D-10 en `docs/specs/00-decisiones.md`). Los WAV **no están en git**; solo este archivo.

## Origen

| Dato | Valor |
|---|---|
| Fecha | 24 de octubre de 2026 |
| Herramienta | `minimax-music3` (del titular) |
| Runtime | audio.cpp (Apache-2.0) |
| Modelo | `minimax-music3`, port GGUF `audio-cpp/MiniMax-Music3-GGUF` (mezcla q4_0) |
| Modelo original | `MiniMaxAI/MiniMax-Music3` |
| Formato | WAV 44,1 kHz, 16 bits, estéreo (~201 MB en total) |

## Licencia

«MiniMax-Music3 Community License»: https://huggingface.co/MiniMaxAI/MiniMax-Music3/blob/main/LICENSE (el port GGUF publica el mismo texto).

- **Permitido**: distribuir y regalar los resultados, incluido el uso comercial. La licencia no reclama ni restringe los resultados.
- **Condición de la excepción**: vale solo mientras la licencia siga permitiéndolo.

## Obligaciones

- [ ] Mostrar «MiniMax-Music3» de forma visible en la interfaz del producto comercial (§3.1).
- [ ] Indicar que la música es **generada por máquina** al difundirla en público (Anexo A, punto 11).
- [ ] Cumplir la política de uso aceptable (Anexo A); MiniMax puede actualizarla.
- [ ] Pedir autorización a api@minimax.io si los ingresos anuales superan 20 millones de USD (§3.2).
- [ ] Conservar el texto de la licencia en `THIRD_PARTY_NOTICES.md`.

El titular asume los riesgos de distribuir los resultados e indemniza a MiniMax si incumple la licencia.

## Derechos de autor

El audio puramente generado por IA puede no ser protegible por copyright en algunas jurisdicciones. No es asesoría legal. No prometer exclusividad sobre estas pistas.

## Pistas con voz detectada (revisar antes de publicar)

La primera tanda marcó palabras en 9 pistas: `bgm-04`, `underscore-02`, `underscore-04`, `underscore-05`, `underscore-06`, `underscore-10`, `soundtrack-08`, `ambient-06`, `ambient-10`.

**Revisadas y resueltas.** Las 9 se regeneraron con otra semilla y se vuelto a comprobar. La verificación ahora transcribe cada pista con tres pistas de idioma distintas (en, zh, es) y exige que las tres devuelvan el mismo texto: el ASR alucina frases sobre una cama instrumental, y esa alucinación cambia según el idioma pedido. Un texto idéntico en los tres significa que los fonemas están de verdad en el audio.

Estado final: **40/40 sin voz**, confirmado con tres rondas independientes de la comprobación por pista. Silero VAD coincidió en todos los casos.

## Siguiente paso

Al implementar la pestaña Sonidos (05, FV-09): convertir a formato comprimido, versionar y registrar en `assets/LICENSES.toml` y `THIRD_PARTY_NOTICES.md`.
