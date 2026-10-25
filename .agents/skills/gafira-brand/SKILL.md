---
name: gafira-brand
description: >
  Apply the Gafira brand guide (colors, typography, logo, voice, UI rules, motion)
  when writing UI code, tokens, copy, splash/icons, docs or specs for Gafira.
  Trigger: UI, theme tokens, colors, fonts, logo or icon usage, microcopy, error
  messages, splash screen, marketing text, or any spec that names visual rules.
---

# Gafira brand rules

Authority, in order: `docs/manuals/marca/manual-gafira.md` (the brand manual), then `docs/specs/backlog/03-diseno-visual-e-interaccion.md` (tokens JSON, dark mode, glossary ES+EN, wireframes, contrast table). If they disagree, the manual wins; report the conflict, do not guess. Re-read the manual section you apply; this file is a checklist, not a copy.

## Name and voice
- Write **Gafira** exactly: no accents, variants or added characters. Never "Canva" or other products' names in specs or UI.
- Slogan ES: **Haz visibles tus ideas.** EN: **Bring your ideas to life.** Secondary: Diseña sin complicarte / Design without the hassle; Empieza a crear / Start creating.
- Short, action-oriented sentences; clear verbs (crear, diseñar, mover, compartir, exportar). Only claim functions that exist. Avoid "revolucionario", "el mejor", "sin límites", "solución integral".
- Errors say what happened and the real corrective action ("No pudimos guardar este cambio. Inténtalo de nuevo"). UI names come from the glossary in spec 03 §2; do not reuse other products' menu names.
- Languages: Spanish and English (D-08); UI strings live in `gafira-i18n` catalogs, never hard-coded.

## Colors (tokens `brand.*`)
| Token | HEX | Use |
|---|---|---|
| ink | `#18243B` | text, logo, navigation, dark backgrounds |
| coral | `#FF6B57` | CTA, accents |
| canvas | `#FAFAF7` | light backgrounds |
| mint | `#3FBFA9` | secondary accents |
| mist | `#E7EAF2` | cards, dividers, neutral surfaces |

- Primary button: coral background with **ink text**. Never white small text on coral (2.80:1). Secondary: ink background with white text, or outline.
- Contrast: 4.5:1 normal text, 3:1 large text and non-text components. Verified pairs: ink/canvas 14.83, white/ink 15.51, ink/coral 5.53, ink/mint 6.82. Check every new pair; use the table in spec 03 §4.
- Proportions are guidance, not per-screen quotas: ~60% light, 25% ink, 10% coral, 5% mint.
- Take hex values from the token source (spec 03 §3), never retype them in code; dark theme uses the derived dark tokens.

## Typography
- Headings/marketing: **Manrope** 600–800. UI/body: **Inter** 400–600. Fallback: Inter.
- Scale: display 56/64 700, h1 40/48 700, h2 30/38 700, h3 24/32 650, body-lg 18/29, body 16/26, body-sm 14/22, caption 12/18 500.
- Fonts must be bundled with the app (fully local); check each font's license before embedding.

## Layout and shape
- 4 px spacing grid, prefer 8 px between usual elements. Radius 8 / 12 / 20 px. Web grid 12 columns (4 on mobile).
- Rectangles with moderate corners, straight cuts, layers; simple stroke icons with consistent weight; tooltip or label when meaning is not obvious.
- The user's content is the protagonist: no permanent watermarks, no big logos in the editor.

## Motion
- 120–220 ms for microinteractions; transitions reinforce spatial relations (insert, reorder, edit, export). Respect reduced motion (`prefers-reduced-motion` / OS setting).

## Logo
- Files live in `docs/manuals/marca/assets/` (primary, dark-background, stacked, symbol, wordmark). Missing variants (mono, symbol-dark, favicon) are listed in spec 03 §5.2: do not invent them; flag them.
- Never retype the name in a font; use the SVG. No stretch, rotate, mirror, shadow, outline, gradient or texture; do not recolor panels or remove one of the three panels; do not change symbol/name spacing.
- Light backgrounds: primary. Ink `#18243B` background: dark-background variant. Wordmark only on light backgrounds.
- Clear space X = 1/5 of the symbol width on every side. Minimums: horizontal 160 px (print 35 mm), stacked 120 px, symbol in UI 24 px (ideal 32–48), favicon 16 px, app icon 192/512 px (own export). Below 160 px use the symbol.
- On busy images put the logo on a smooth contrast panel. Splash: logo and tagline on the warm-white panel `#FAFAF7` (spec 03 §7.10), thin coral progress bar.

## Flow and accessibility
- Product flow: **Elegir → Personalizar → Compartir**. Keyboard navigation, visible focus, readable contrast, text alternatives, comfortable targets, undo/redo.

## Checklist before finishing
1. Colors and fonts come from tokens, not literals.
2. Every text/background pair is in the verified table or was computed.
3. Copy follows the voice rules and exists in ES and EN.
4. Logo variant matches the background; clear space and minimum size respected.
5. Motion respects reduced motion.
6. Anything the manual or spec 03 leaves open is reported as a question, not decided.
