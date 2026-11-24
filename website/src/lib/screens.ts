// The screenshots of Gafira in each language (the app's own UI, captured from
// version 0.2.0).
import type { ImageMetadata } from 'astro';
import editorEs from '../assets/screens/editor-es.png';
import homeEs from '../assets/screens/home-es.png';
import exportEs from '../assets/screens/export-es.png';
import editorEn from '../assets/screens/editor-en.png';
import homeEn from '../assets/screens/home-en.png';
import exportEn from '../assets/screens/export-en.png';
import type { Locale } from '../i18n/ui';

export interface Screens {
  editor: ImageMetadata;
  home: ImageMetadata;
  exporting: ImageMetadata;
}

export const screens: Record<Locale, Screens> = {
  es: { editor: editorEs, home: homeEs, exporting: exportEs },
  en: { editor: editorEn, home: homeEn, exporting: exportEn },
};
