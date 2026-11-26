// The screenshots of Gafira in each language (the app's own UI, captured from
// version 0.2.0; the 3D screens from 0.3.0).
import type { ImageMetadata } from 'astro';
import editorEs from '../assets/screens/editor-es.png';
import homeEs from '../assets/screens/home-es.png';
import exportEs from '../assets/screens/export-es.png';
import editorEn from '../assets/screens/editor-en.png';
import homeEn from '../assets/screens/home-en.png';
import exportEn from '../assets/screens/export-en.png';
import packsEs from '../assets/screens/packs-es.webp';
import packsEn from '../assets/screens/packs-en.webp';
import model3dEs from '../assets/screens/model3d-es.webp';
import model3dEn from '../assets/screens/model3d-en.webp';
// Hero slides added later; the flowers capture exists only in Spanish and is shared.
import flowersEs from '../assets/screens/flowers-es.webp';
import model3dDragonEs from '../assets/screens/model3d-dragon-es.webp';
// The canvas close-up of the 3D editor, one capture for both languages.
import model3dDetail from '../assets/screens/model3d-detail.webp';
import type { Locale } from '../i18n/ui';

export interface Screens {
  editor: ImageMetadata;
  home: ImageMetadata;
  exporting: ImageMetadata;
  /** The editor with the three illustration packs installed. */
  packs: ImageMetadata;
  /** The editor with a 3D model being rotated (version 0.3.0). */
  model3d: ImageMetadata;
  /** The editor with the Botanical Flowers pack on the page. */
  flowers: ImageMetadata;
  /** The 3D screen shown in the hero carousel. */
  model3dHero: ImageMetadata;
  /** A close-up of the canvas with the 3D gizmos and tool bar. */
  model3dDetail: ImageMetadata;
}

export const screens: Record<Locale, Screens> = {
  es: {
    editor: editorEs,
    home: homeEs,
    exporting: exportEs,
    packs: packsEs,
    model3d: model3dEs,
    model3dDetail,
    flowers: flowersEs,
    model3dHero: model3dDragonEs,
  },
  en: {
    editor: editorEn,
    home: homeEn,
    exporting: exportEn,
    packs: packsEn,
    model3d: model3dEn,
    model3dDetail,
    flowers: flowersEs,
    model3dHero: model3dEn,
  },
};
