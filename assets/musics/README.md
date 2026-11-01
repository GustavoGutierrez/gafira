# Biblioteca de música de Gafira

40 pistas instrumentales originales para edición de video: **10 Background Music (BGM)**,
**10 Underscore**, **10 Soundtrack** y **10 Ambient**. Todas se generaron desde cero con
[MiniMax Music 3](https://huggingface.co/MiniMaxAI/MiniMax-Music3) y no arrastranAtribución de
catálogos de terceros.

## Uso rápido

1. Elige una categoría de abajo y abre el WAV correspondiente.
2. Ponlo en la línea de tiempo y recórtalo o repítelo: cada pista es una cama independiente.
3. Si quieres otra versión, regenera con otra semilla (ver [Regenerar](#regenerar)).

## Especificaciones técnicas

| Propiedad | Valor |
|---|---|
| Formato | WAV, PCM sin comprimir |
| Frecuencia de muestreo | 44 100 Hz |
| Profundidad de bits | 16 bits |
| Canales | Estéreo |
| Duración | 24,8 – 30,0 s por pista |
| Tamaño | ≈ 5,0 MB por pista |
| Voces | **Ninguna** — todas son instrumentales (ver [Control de calidad](#control-de-calidad)) |
| Bucles | El modelo cierra la mayoría en una frase: revisa la cola antes de repetir |

## Categorías

| Categoría | Qué es | Cómo usarla |
|---|---|---|
| **BGM** | Música que acompaña el video sin ser la protagonista | Cama de fondo completa bajo diálogo, vlogs, productos, montajes |
| **Underscore** | Instrumental bajo narración, diálogo o escena | Narración, explicadores, entrevistas, documentales: deja espacio a la voz |
| **Soundtrack** | La banda sonora de una película o producción audiovisual | Tráilers, cartas de título, clímax, transiciones: lleva el drama ella sola |
| **Ambient** | Música ambiental que crea un espacio sin distraer | Tonos de fondo continuos bajo planos largos,.settings de lugar, relleno |

---

## Background Music (BGM)

Música que acompaña sin robar protagonismo.

| Archivo | Qué es | Sonido | Mejor para | Términos de búsqueda |
|---|---|---|---|---|
| `bgm-01-neon-drive.wav` | Conduciendo de noche por una ciudad de neón | Synthwave, bajo analógico, arpegios y pads, 110 BPM, reverb amplio | Videos de auto, ciudad nocturna, tecnología retro, cyberpunk | synthwave, retro, noche, neón, conducir, años 80, cyberpunk, electrónico |
| `bgm-02-morning-coffee.wav` | Un arranque lento del día en una mesa de café | Lo-fi hip hop, drum machine polvorienta, rhodes apagados, 82 BPM, crujido de vinilo | Vlogs de estudio, rutina, edición tranquila, contenido ASMR | lofi, chill, estudio, café, relax, mañana, vinilo, hip hop |
| `bgm-03-hustle-street.wav` | La energía de un mercado callejero | Afrobeat, groove de talking drum, guitarra eléctrica limpia, 104 BPM | Viajes, comida, cultura urbana, energía deportiva | afrobeat, africano, mercado, viaje, comida, movido, groove, mundo |
| `bgm-04-gym-momentum.wav` | Un montaje de entrenamiento a tope | Trap, sub 808 contundente, hi-hats crocantes, 140 BPM, mezcla dura | Fitness, entrenamientos, energía deportiva, transformaciones | trap, gimnasio, entrenamiento, motivación, deporte, hype, 808, rápido |
| `bgm-05-sunday-market.wav` | Una vuelta tranquila por el mercado del domingo | Bossa nova, guitarra de nylon, percusión suave con cepillos, 132 BPM | Lifestyle, cocina, vlogs de viaje, años grises con buena onda | bossa nova, brasil, café, lifestyle, cocina, cálido, relajado |
| `bgm-06-paper-plane.wav` | La energía de un cuarto con grabación casera | Indie pop, guitarra eléctrica con mucho coro, palmas, 118 BPM | Juventud, nostalgia, vlogs, montajes alegres | indie pop, coro brillante, soleado, juventud, nostálgico, animado, guitarra |
| `bgm-07-deep-focus.wav` | Espacio mental para el trabajo profundo | Electrónica ambiental minimalista, pads analógicos suaves, arpegio filtrado lento, 72 BPM | Programación, tutoriales, trabajo profundo, productividad | minimal, focus, código, calma, lofi, ambiental, productividad, suave |
| `bgm-08-rooftop-sunset.wav` | Hora dorada sobre la ciudad | Chillhop, acordes de Rhodes jazz, batería relajada, 88 BPM, calor de cinta | Atardeceres, vlogs lentos, montajes reflexivos | chillhop, jazzoso, atardecer, hora dorada, relajado, chill, vinilo |
| `bgm-09-neon-noir.wav` | Tensión noir en una habitación oscura | Electrónica cinematográfica oscura, bajo analógico desafinado, percusión metálica dispersa, 96 BPM | Thriller, noir tecnológico, revelaciones de producto, suspense | noir, oscuro, tenso, thriller, cinematográfico, electrónico, suspense, sombrío |
| `bgm-10-first-light.wav` | Un toma cálido, cercano e íntimo | Neo-soul, piano de rhodes cálido, guitarra con dedos, 84 BPM, calor de cinta | Entrevistas, lifestyle, contenido emotivo, branding | neo soul, cálido, íntimo, soul, amable, piano, acogedor, nostálgico |

## Underscore

Camas instrumentales escritas paraGigelow una voz.

| Archivo | Qué es | Sonido | Mejor para | Términos de búsqueda |
|---|---|---|---|---|
| `underscore-01-doc-tension.wav` | Tensión documental que crece despacio | Piano de fieltro contenido, drone de cuerdas graves, 70 BPM, disperso y moderno | Documental, crimen real, investigación, revelación lenta | documental, tensión, grows, investigación, serio, disperso, piano, drone |
| `underscore-02-explainer.wav` | Una cama neutra que no estorba | Piano eléctrico apagado y limpio, percusión suave de mazos, 90 BPM | Explicadores, corporativo, tutoriales, noticias | explicador, corporativo, neutro, limpio, presentación, tutorial, sutil |
| `underscore-03-tech-reveal.wav` | Un producto que aparece | Arpegio de synth pulsante, percusión digital nítida, 112 BPM | Reseñas de tecnología, lanzamientos, demos de apps | tecnología, lanzamiento, reveal, digital, moderno, pulso, futurista |
| `underscore-04-countdown.wav` | Esperando a que algo empiece | Percusión de reloj, cuerdas filtradas en ascenso, 100 BPM | cuentas atrás,advance trailers, revelaciones, momentos de suspense | cuenta atrás, tic, anticipación, tráiler, suspense, crescendo, reloj |
| `underscore-05-reflective.wav` | Una respuesta meditada | Guitarra acústica sola, violonchelo suave, 68 BPM, cálido y reflexivo | Entrevistas, ensayos, memorias, historias personales | reflexivo, meditado, entrevista, acústico, cálido, contemplativo, amable |
| `underscore-06-news-sting.wav` | Una cama sobria de emisión | Cuerdas graves contenidas, pulso de tic discreto, 95 BPM |-noticias, reportes, anuncios formales | noticias, emisión, serio, autoritativo, formal, sobrio, sting |
| `underscore-07-medical-calm.wav` | Tranquilidad en una sala clínica | Texturas de campana cristalina, piano suave, 76 BPM, sin percusión | Salud, bienestar, explicadores de ciencia, cuidado | médico, salud, calma, tranquila, bienestar, ciencia, amable, suave |
| `underscore-08-history-reel.wav` | Archivos de otra década | Cuerdas de época, tambores suaves de marco, 88 BPM | Historia, archivo, drama de época, documentales | historia, archivo, vintage, época, documental, patrimonio, antiguo |
| `underscore-09-data-flow.wav` | Números avanzando | Synth pulsado mínimo, pulso filtrado constante, 104 BPM | Visualización de datos, analítica, infografía, IA | datos, analítica, tecnología, infografía, minimal, limpio, progreso, gráficas |
| `underscore-10-fade-out.wav` | Una frase final que se asienta | Piano que se desvanece, oleada de cuerdas cálida, 60 BPM | Cartas de fin, cierres, créditos, cierres amables | cierre, final, fade out, salida, créditos, amable, resolución, calma |

## Soundtrack

Bandas sonoras que llevan el drama por sí solas.

| Archivo | Qué es | Sonido | Mejor para | Términos de búsqueda |
|---|---|---|---|---|
| `soundtrack-01-hero-rise.wav` | El momento en que aparece el héroe | Orquestal épico, fanfarria de metales, timpani, 92 BPM, heroico y amplio | Ediciones de superhéroe, tráileres, grandes revelaciones, gaming | épico, heroico, orquestal, tráiler, superhéroe, triunfal, metales, fanfarria |
| `soundtrack-02-dark-horror.wav` | Algo se mueve en la oscuridad | Terror, cuerdas graves disonantes, metal rasgado, 66 BPM, acecho | Cortos de terror, intros de thriller, Halloween | terror, oscuro, acecho, inquietante, tensión, terrorífico, cuerdas |
| `soundtrack-03-tense-chase.wav` | Corriendo contra el reloj | Persecución de acción, cuerdas ostinato en movimiento, percusión incansable, 148 BPM | Escenas de persecución, acción, fallos deportivos | acción, persecución, rápido, urgente, escapar, thriller, motor, percusión |
| `soundtrack-04-emotional-heart.wav` | La despedida | Drama emocional, violín solo, piano suave, 64 BPM, tierno y doloroso | Despedidas, rupturas, momentos tristes, drama | emocional, triste, conmovedor, violín, piano, tierno, drama, agridulce |
| `soundtrack-05-adventure.wav` | Partir hacia lo desconocido | Aventura, sección de metales audaz, percusión en marcha, 120 BPM | Viajes, exploración, deporte, naturaleza | aventura, audaz, metales, viaje, exploración, optimista, cinematográfico |
| `soundtrack-06-sci-fi-world.wav` | Un lugar que no es el nuestro | Sci-fi, drones de synth analógicos profundos, pulsos metálicos procesados, 80 BPM | Sci-fi, espacio, tecnología, construcción de mundos | ciencia ficción, espacio, drone, alienígena, futurista, cósmico, otro mundo |
| `soundtrack-07-battle.wav` | Dos bandos chocan | Batalla medieval, tambores de marco, metales graves, 132 BPM, feroz | Gaming, historia, combate, fantasía | batalla, medieval, guerra, tambores, metales, feroz, fantasía, épico |
| `soundtrack-08-mystery.wav` | Una pregunta que vale la pena | Misterio, cuerdas pizzicato, maderas curiosas, 84 BPM, juguetón | Misterio, detectives, acertijos, explicadores | misterio, pizzicato, curioso, juguetón, detective,-Canada, intriga |
| `soundtrack-09-triumphant.wav` | Funcionó | Triunfal, conjunto completo de cuerdas, metales brillantes, 108 BPM, celebratorio | Victorias, logros, premios, hitos | triunfal, victoria, celebración, logro, ascendente, metales, conjunto |
| `soundtrack-10-melancholy-farewell.wav` | Aceptación, no derrota | Agridulce, cuerdas apagadas, piano solo, 58 BPM, tristeza amable | Finales, homenajes, memorials, cierres reflexivos | agridulce, melancólico, despedida, homenaje, tristeza, amable, piano, cuerdas |

## Ambient

Camas atmosféricas que crean un lugar sin robar atención.

| Archivo | Qué es | Sonido | Mejor para | Términos de búsqueda |
|---|---|---|---|---|
| `ambient-01-rain-room.wav` | Lluvia contra la ventana, adentro | Lavado de ruido filtrado, truenos lejanos y apagados, evolución muy lenta | Escenas de lluvia, contemplación interior, contenido acogedor | lluvia, tormenta, ventana, interior, acogedor, día gris, atmósfera |
| `ambient-02-forest-dawn.wav` | La primera luz en el bosque | Pad sostenido cálido, texturas de pájaro lejanas, extremadamente lento | Naturaleza, time-lapse de naturaleza, naturaleza calma, meditación | bosque, pájaros, amanecer, naturaleza, calma, bosque, pacífico, amanecer |
| `ambient-03-deep-space.wav` | Espacio vacío entre estrellas | Tonos de synth muy graves y sostenidos, modulación lenta, mínimo y vasto | Espacio, cosmos, sci-fi, meditación | espacio, cosmos, espacio profundo, drone, vasto, mínimo, estrellas, cósmico |
| `ambient-04-cafe-hum.wav` | Un café lleno, oído desde dentro | Murmullo apagado, zumbido suave de máquina de espresso | Vlogs de café, trabajar desde el café, vida urbana | café, cafetería, charla, lleno, trabajo, urbano, ambiente |
| `ambient-05-underwater.wav` | Bajo la superficie | Drone sub grave apagado, burbujas dispersas, muy lento | Bajo el agua, buceo, marino, surreal | bajo el agua, océano, sub, burbujas, profundo, marino, sumergido, acuático |
| `ambient-06-night-city.wav` | Una ciudad que nunca duerme | Zumbido de tráfico lejano, bruma eléctrica suave, continuo y algo solitario | Nocturno urbano, ciudad de noche, soledad urbana, cyberpunk | ciudad nocturna, tráfico, urbano, solo, zumbido, calle, cyberpunk, metrópoli |
| `ambient-07-fireplace.wav` | Un fuego en una habitación callada | Textura de brasas crepitantes, calidez baja de habitación | Acogedor, invierno, lectura, contenido de hogar | chimenea, fuego, crepitar, acogedor, invierno, cálido, hogar, íntimo |
| `ambient-08-snow-field.wav` | Nada más que frío y blanco | Textura de viento aéreo, brillos lejanos agudos, muy disperso y quieto | Invierno, paisajes mínimos, silencio, contemplación | nieve, invierno, viento, frío, quieto, mínimo, vasto, sereno, vacío |
| `ambient-09-machine-room.wav` | Máquinas funcionando | Zumbido eléctrico constante, pulso tenue de ventilador, mecánico y calmo | Tecnología, servidores, industria, b-roll | servidor, máquina, eléctrico, zumbido, industrial, tecnología, ventilador, mecánico |
| `ambient-10-warm-drone.wav` | Un zumbido largo y suave que deriva | Pad analógico sostenido, warble de cinta amable, muy lento e hipnótico | Meditación, relleno largo, transiciones lentas, sueño | drone, cálido, hipnótico, meditación, sueño, sostenido, deriva, relajante |

---

## Control de calidad

Cada pista se verificó **libre de voces** antes de entregarse. La comprobación no es una sola
transcripción, porque el ASR alucina frases sobre música instrumental.

La verificación transcribe cada pista con tres pistas de idioma distintas (en, zh, es) y exige que
**las tres devuelvan el mismo texto**:

| Resultado en los tres idiomas | Veredicto | Significado |
|---|---|---|
| Los tres vacíos | **limpio** | No hay voz |
| Los tres idénticos y no vacíos | **voces** | Voz real: los fonemas están en el audio |
| Cualquier otro caso | **sospechoso** | Un solo idioma no es evidencia: el decodificador proyecta sobre el idioma pedido |

Estado final de la biblioteca: **40/40 limpias**, confirmado con tres rondas independientes de la
comprobación por pista. Las pistas que fallaron se regeneraron con otra semilla y se volvieron a
comprobar.

Silero VAD se usó como señal secundaria y coincidió en todos los casos.

## Regenerar

El generador vive fuera del repositorio, en `AI/minimax-music3/` dentro de tu carpeta de
proyectos. Ejecuta MiniMax Music 3 en local con el runtime audio.cpp: sin clave de API, sin
costo y sin límite de peticiones.

```bash
cd ../AI/minimax-music3
./scripts/start_server.sh &                       # servidor local de música en :8080
python3 scripts/generate_music.py \
  --manifest tracks.json --out-dir <esta carpeta>   # regenera la biblioteca completa
```

`tracks.json` guarda el prompt, la duración y la semilla de cada pista, así que cambiar una sola
semilla produce una versión nueva de esa pista conservando su carácter.

Para arreglar una pista que salió con voz, cambia su **semilla**: el modelo está condicionado a
letra y la semilla decide la trayectoria muestreada.

```bash
python3 scripts/repair_tracks.py --manifest tracks.json --out-dir <esta carpeta> --fails fails.txt
```

## Búsqueda

Los archivos se nombran `<categoría>-<número>-<slug>.wav`, así que el nombre ya dice la
categoría. Para buscar por estado de ánimo o instrumento, usa la columna **Términos de búsqueda**
de arriba: son los términos que alguien escribiría de verdad en un panel de biblioteca.

Cada pista es una cama completa de 30 segundos, no un bucle: abre, se desarrolla y resuelve.
Recorta la cola, o dale la vuelta, para que cierre limpio.

## Procedencia y licencia

| Elemento | Detalle |
|---|---|
| Modelo | `MiniMaxAI/MiniMax-Music3`, pesos abiertos, ejecutado en local |
| Runtime | [audio.cpp](https://github.com/0xShug0/audio.cpp), Apache-2.0 |
| Licencia del modelo | MiniMax-Music3 Community License: revisa los términos antes de una publicación comercial |
| Audio | Generado solo a partir de prompts de texto. No se usó material de origen con licencia ni material protegido |
| Excepción de Gafira | D-10 en `specs/00-decisiones.md`. Detalle en `LICENSE.md` |
