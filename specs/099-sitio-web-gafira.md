# 099 — Sitio web de Gafira (landing)

> **Estado:** in-progress — 2026-11-24 — publicado en https://gafira.com (Cloudflare Workers, primera versión desplegada y verificada: páginas 200, 404 propia, cabeceras de caché y seguridad; `www` → `gafira.com` con 301 que conserva ruta y consulta, y http → https); pendiente la revisión del titular.

## Problema y resultado esperado

- Gafira no tiene una página pública que la presente ni un lugar estable desde donde descargarla.
- Resultado: una landing en `gafira.com`, en español e inglés, con las funciones que existen, capturas reales de la app y las descargas de la última versión publicada en GitHub.

## Decisión técnica

- **Astro (estático), no VitePress.** VitePress está pensado para documentación y envía su tema y su runtime de Vue a cada página; Astro envía cero JavaScript por defecto (solo dos scripts pequeños: revelado/tema y detección del sistema), optimiza las capturas a AVIF/WebP y trae i18n y sitemap. La documentación, si llega, puede ser otra sección o sitio.
- **Carpeta `website/`** en la raíz: `apps/*` es miembro del workspace de Cargo.
- **Cloudflare Workers con recursos estáticos** (igual que el sitio de referencia del titular): dominio propio `gafira.com`, sin script de Worker; `www.gafira.com` → `gafira.com` con una Redirect Rule 301 de la zona que conserva ruta y consulta.

## Requisitos

- **S99-01 Idiomas.** Español en `/` e inglés en `/en/`, con selector, `hreflang` y sitemap con alternativas. Todos los textos en `src/i18n/ui.ts` con un tipo común (falta un texto = error de `astro check`).
- **S99-02 Marca.** Colores y radios leídos de `assets/theme/gafira.tokens.json`; Manrope en títulos e Inter en texto; botón principal coral con texto tinta; logotipo principal en claro y la variante de fondo oscuro en oscuro; favicon y símbolo del manual; iconos de toque y del manifiesto desde `assets/app-icon/`.
- **S99-03 Modo claro y oscuro.** Sigue al sistema por defecto; el botón de la cabecera lo cambia y lo recuerda (`localStorage`), aplicado antes del primer pintado. Colores de cada modo desde los tokens `modes.light` y `modes.dark`.
- **S99-04 Contenido verdadero.** Solo funciones que existen; cifras comprobadas en el código (75 plantillas por idioma, 215 formas en 17 secciones, 33 familias tipográficas, 16 pinceles). Capturas reales de Gafira 0.2.0 en cada idioma (la app en español para `/` y en inglés para `/en/`, también en la imagen para compartir).
- **S99-05 Descargas.** Al compilar se lee la última versión de GitHub y se ordenan sus paquetes por sistema (instalador y .zip de Windows, .dmg de macOS, .deb y AppImage de Linux), con tamaño, notas, sumas SHA-256 y todas las versiones; sin red se usa `src/data/release-fallback.json`. El botón principal apunta al paquete del sistema del visitante y marca su tarjeta («Para tu equipo»); en móvil o sin JavaScript lleva a la sección de descargas.
- **S99-06 Movimiento.** Entrada escalonada del hero, ventana que se endereza al desplazar (donde hay animaciones por desplazamiento), chips flotantes, gráfico animado del flujo Elegir → Personalizar → Compartir sincronizado con las tarjetas de pasos, revelado de pasos, capturas y descargas, sombra de la cabecera al desplazar. Todo se detiene con `prefers-reduced-motion`.
- **S99-07 Accesibilidad y SEO.** HTML semántico (un `h1`, h2 → h3 sin saltos, `header`/`nav`/`main`/`footer`, cada `section` nombrada por su encabezado, `ol` para los pasos, `time` para la fecha), enlace «Saltar al contenido», foco visible, textos alternativos y contraste verificado. Títulos (≤ 60) y descripciones (≤ 155) con los términos que se buscan, `robots` con `max-image-preview:large`, `canonical`, `hreflang` con `x-default`, Open Graph y Twitter completos con la captura de cada idioma, JSON-LD de `Organization`, `WebSite` y `SoftwareApplication` (versión, descarga, captura; sin precio ni valoraciones inventadas), sitemap con `lastmod` y alternativas, `robots.txt`, 404 bilingüe con `noindex`, precarga de fuentes y caché inmutable para `/_astro/*`.
- **S99-08 Paquetes gratis.** Sección `#packs` bajo las descargas con tres paquetes `.gafpack` de 30 ilustraciones: «Clip art navideño», «Ornamentos navideños» y «Ornamentos para toda ocasión», los tres desde `cdn.gafira.com/packages/` con su tamaño; un paquete con `url: null` en `Packs.astro` se muestra como «Muy pronto». Cada tarjeta muestra cinco ilustraciones del paquete: al aparecer salen apiladas del centro y se abren en abanico, luego flotan y se separan al apuntar o enfocar; el botón confirma «Descarga iniciada» y lo anuncia a lectores de pantalla. Debajo, los pasos Descargar → Instalar (doble clic o Inicio → Instalar paquete) → Diseñar (Elementos → Ilustraciones) y la nota de licencia. Movimiento detenido con `prefers-reduced-motion`.

## Pruebas y evidencia

- `npm run build` (incluye `astro check`): 0 errores.
- Revisión en navegador a 1440×900 y 390×844, ES y EN, claro y oscuro; sin desbordamiento horizontal.
- Pasada `polish` del skill `impeccable` y su detector: sin hallazgos.

## Pendiente

- Revisión del titular.
- Dar de alta el sitio en Google Search Console y enviar el sitemap.
- Tras cada versión nueva: volver a compilar y desplegar el sitio.
