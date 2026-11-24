// Every text of the landing page in Spanish and English (D-08). Both
// languages share the `Copy` shape, so a missing text fails `astro check`.
// Voice: short sentences, clear verbs, only functions Gafira has (brand
// manual §voice).

export type Locale = 'es' | 'en';

export const locales: Locale[] = ['es', 'en'];

export interface Feature {
  icon: string;
  title: string;
  text: string;
}

export interface Copy {
  meta: { title: string; description: string; ogAlt: string };
  nav: {
    features: string;
    how: string;
    download: string;
    skip: string;
    language: string;
    menu: string;
    theme: string;
  };
  hero: {
    title: string;
    lead: string;
    download: string;
    downloadFor: string;
    features: string;
    platforms: string;
    screenAlt: string;
    chips: { background: string; colors: string; exported: string };
  };
  how: {
    title: string;
    steps: { title: string; text: string }[];
    motion: { alt: string; title: string; subtitle: string; cta: string };
  };
  features: { title: string; lead: string; items: Feature[] };
  showcase: {
    title: string;
    items: { title: string; text: string; points: string[]; alt: string }[];
  };
  local: { title: string; text: string; points: Feature[] };
  download: {
    title: string;
    lead: string;
    version: string;
    published: string;
    recommended: string;
    windows: { name: string; installer: string; installerNote: string; zip: string; zipNote: string; note: string };
    macos: { name: string; dmg: string; dmgNote: string; note: string };
    linux: { name: string; deb: string; debNote: string; appImage: string; appImageNote: string; note: string };
    checksums: string;
    notes: string;
    all: string;
  };
  cta: { title: string; text: string; button: string };
  footer: { tagline: string; product: string; releases: string; rights: string; language: string };
  notFound: { title: string; text: string; home: string };
}

const es: Copy = {
  meta: {
    title: 'Gafira · Haz visibles tus ideas',
    description:
      'Gafira es un programa de diseño gráfico para todas las personas y creadores de contenido. Elige una plantilla, personalízala y compártela. Para Windows, macOS y Linux.',
    ogAlt: 'El editor de Gafira con una plantilla abierta',
  },
  nav: {
    features: 'Funciones',
    how: 'Cómo funciona',
    download: 'Descargar',
    skip: 'Saltar al contenido',
    language: 'Idioma',
    menu: 'Menú',
    theme: 'Cambiar entre modo claro y oscuro',
  },
  hero: {
    title: 'Haz visibles tus ideas.',
    lead: 'Gafira es un programa de diseño para crear publicaciones, presentaciones, invitaciones e impresos en minutos. Elige una plantilla, personalízala y compártela, desde tu equipo.',
    download: 'Descargar Gafira',
    downloadFor: 'Descargar para',
    features: 'Ver funciones',
    platforms: 'Windows · macOS · Linux',
    screenAlt: 'El editor de Gafira: una publicación de cafetería abierta, con el panel Elementos y los colores del diseño',
    chips: { background: 'Fondo quitado', colors: 'Colores del diseño', exported: 'Exportado en PDF' },
  },
  how: {
    title: 'De la idea al diseño en tres pasos',
    steps: [
      {
        title: 'Elige',
        text: 'Empieza con una plantilla para redes sociales, presentaciones, impresos, invitaciones, documentos o web, o con un tamaño propio.',
      },
      {
        title: 'Personaliza',
        text: 'Cambia textos, colores, fotos y formas. Arrastra, alinea y ordena las capas hasta que se vea como lo imaginas.',
      },
      {
        title: 'Comparte',
        text: 'Exporta en PNG, JPEG, WebP, AVIF, SVG, PDF o PowerPoint, listo para publicar o imprimir.',
      },
    ],
    motion: {
      alt: 'Animación: un diseño se arma con tres paneles, cambia de color, recibe sus textos y se exporta.',
      title: 'Gran apertura',
      subtitle: 'Este sábado',
      cta: 'Ven',
    },
  },
  features: {
    title: 'Todo lo que necesitas para diseñar',
    lead: 'Herramientas claras para crear sin complicarte, desde una publicación hasta una presentación completa.',
    items: [
      {
        icon: 'layout',
        title: 'Plantillas y tamaños',
        text: '75 plantillas para Instagram, presentaciones, carteles, invitaciones, documentos y correo. Busca por tipo de diseño o crea uno con tu tamaño.',
      },
      {
        icon: 'shapes',
        title: 'Formas, líneas y comillas',
        text: '215 formas: polígonos, estrellas, flechas, diagramas, globos de diálogo, nubes y comillas. Líneas que se conectan a las formas.',
      },
      {
        icon: 'text-aa',
        title: 'Texto con estilo',
        text: '33 familias tipográficas incluidas, combinaciones de fuentes, texto curvo, texto dentro de las formas y efectos de texto.',
      },
      {
        icon: 'eyedropper',
        title: 'Color en cada detalle',
        text: 'Gotero con lupa, colores sólidos y degradados, los colores de tu diseño y paletas predeterminadas.',
      },
      {
        icon: 'magic-wand',
        title: 'Fotos e imágenes',
        text: 'Ajustes, filtros, sombras y recortes con formas. Quita el fondo, mejora la resolución o borra objetos con modelos que funcionan en tu equipo.',
      },
      {
        icon: 'images',
        title: 'Fotos, iconos e ilustraciones',
        text: 'Busca fotos e ilustraciones libres desde el panel Elementos e importa tus imágenes, SVG o PDF con arrastrar y soltar.',
      },
      {
        icon: 'chart-bar',
        title: 'Tablas, gráficos y QR',
        text: 'Tablas editables (importa Excel o CSV), gráficos con su editor de datos y códigos QR con colores y logotipo.',
      },
      {
        icon: 'palette',
        title: 'Kits de marca',
        text: 'Guarda logotipos, colores, estilos de texto y fuentes en un kit y aplícalos a cualquier diseño. Compártelo en un archivo .gafkit.',
      },
      {
        icon: 'paint-brush',
        title: 'Dibujo a mano',
        text: '16 pinceles con estilo propio y trazos vectoriales que puedes mover y editar como cualquier elemento.',
      },
      {
        icon: 'stack',
        title: 'Capas y precisión',
        text: 'Capas que se arrastran, bloquean y ocultan; guías, imán, reglas, alinear y distribuir.',
      },
      {
        icon: 'export',
        title: 'Exporta como necesites',
        text: 'PNG, JPEG, WebP y AVIF; SVG; PDF vectorial con CMYK y contraseña; PowerPoint editable.',
      },
      {
        icon: 'package',
        title: 'Paquetes de recursos',
        text: 'Instala paquetes .gafpack con ilustraciones, iconos, formas, marcos y plantillas, y desinstálalos cuando quieras.',
      },
    ],
  },
  showcase: {
    title: 'Así se ve Gafira',
    items: [
      {
        title: 'Un editor claro y directo',
        text: 'Lo que necesitas está a la vista: los elementos a la izquierda, tu diseño al centro y sus propiedades a la derecha.',
        points: ['Colores del diseño y gotero', 'Formas con un clic o arrastrando', 'Deshacer y rehacer en cada paso'],
        alt: 'El editor de Gafira con el panel Elementos abierto y las propiedades de una forma',
      },
      {
        title: 'Empieza por una plantilla',
        text: 'Inicio reúne tus diseños recientes y plantillas por tipo, con un buscador para encontrar la que necesitas.',
        points: ['Redes sociales, presentaciones e impresos', 'Tamaño propio cuando lo necesites', 'Todo se puede cambiar'],
        alt: 'La pantalla Inicio de Gafira con tipos de diseño y plantillas de Instagram',
      },
      {
        title: 'Exporta en el formato justo',
        text: 'Ve la vista previa, elige formato, páginas, escala y calidad, y recuerda tu configuración para la próxima vez.',
        points: ['Imagen, SVG, PDF y PowerPoint', 'Fondo transparente', 'Exporta en segundo plano'],
        alt: 'El diálogo Exportar de Gafira con la vista previa del diseño',
      },
    ],
  },
  local: {
    title: 'Funciona en tu equipo',
    text: 'Gafira es una aplicación de escritorio. Tus proyectos se guardan en archivos .gaf en tu equipo, y las herramientas de imagen con IA funcionan en local.',
    points: [
      {
        icon: 'hard-drives',
        title: 'Archivos tuyos',
        text: 'Un archivo .gaf guarda el diseño, sus imágenes y sus versiones.',
      },
      {
        icon: 'shield-check',
        title: 'IA en tu equipo',
        text: 'Quitar el fondo o mejorar la resolución no sube tus imágenes a ningún servidor.',
      },
      {
        icon: 'clock-counter-clockwise',
        title: 'Versiones y recuperación',
        text: 'Guarda versiones dentro del proyecto y recupera tu trabajo si algo se cierra.',
      },
    ],
  },
  download: {
    title: 'Descarga Gafira',
    lead: 'Elige tu sistema. Las descargas vienen de la publicación oficial en GitHub.',
    version: 'Versión',
    published: 'Publicada el',
    recommended: 'Para tu equipo',
    windows: {
      name: 'Windows',
      installer: 'Instalador',
      installerNote: '.exe · 64 bits',
      zip: 'Versión portable',
      zipNote: '.zip · sin instalar',
      note: 'Si Windows muestra un aviso de SmartScreen, elige «Más información» y luego «Ejecutar de todas formas».',
    },
    macos: {
      name: 'macOS',
      dmg: 'Imagen de disco',
      dmgNote: '.dmg · Apple Silicon',
      note: 'Aún no está notarizada: la primera vez, abre Gafira con clic derecho y elige «Abrir».',
    },
    linux: {
      name: 'Linux',
      deb: 'Paquete .deb',
      debNote: 'Ubuntu, Debian y derivadas',
      appImage: 'AppImage',
      appImageNote: 'Cualquier distribución x86-64',
      note: 'El AppImage necesita permiso de ejecución: chmod +x Gafira-*.AppImage',
    },
    checksums: 'Sumas SHA-256',
    notes: 'Notas de la versión',
    all: 'Todas las versiones',
  },
  cta: {
    title: 'Empieza a crear',
    text: 'Descarga Gafira y convierte tu próxima idea en un diseño listo para compartir.',
    button: 'Descargar Gafira',
  },
  footer: {
    tagline: 'Haz visibles tus ideas.',
    product: 'Producto',
    releases: 'Versiones en GitHub',
    rights: 'Todos los derechos reservados.',
    language: 'Idioma',
  },
  notFound: {
    title: 'No encontramos esta página',
    text: 'Puede que el enlace haya cambiado. Vuelve al inicio para seguir.',
    home: 'Ir al inicio',
  },
};

const en: Copy = {
  meta: {
    title: 'Gafira · Bring your ideas to life',
    description:
      'Gafira is graphic design software for everyone and for content creators. Pick a template, make it yours and share it. For Windows, macOS and Linux.',
    ogAlt: 'The Gafira editor with a template open',
  },
  nav: {
    features: 'Features',
    how: 'How it works',
    download: 'Download',
    skip: 'Skip to content',
    language: 'Language',
    menu: 'Menu',
    theme: 'Switch between light and dark mode',
  },
  hero: {
    title: 'Bring your ideas to life.',
    lead: 'Gafira is design software to create posts, presentations, invitations and prints in minutes. Pick a template, make it yours and share it, right from your computer.',
    download: 'Download Gafira',
    downloadFor: 'Download for',
    features: 'See features',
    platforms: 'Windows · macOS · Linux',
    screenAlt: 'The Gafira editor: a coffee shop post open, with the Elements panel and the design colours',
    chips: { background: 'Background removed', colors: 'Design colours', exported: 'Exported to PDF' },
  },
  how: {
    title: 'From idea to design in three steps',
    steps: [
      {
        title: 'Pick',
        text: 'Start from a template for social media, presentations, prints, invitations, documents or the web, or from your own size.',
      },
      {
        title: 'Customise',
        text: 'Change texts, colours, photos and shapes. Drag, align and order layers until it looks the way you pictured it.',
      },
      {
        title: 'Share',
        text: 'Export to PNG, JPEG, WebP, AVIF, SVG, PDF or PowerPoint, ready to post or print.',
      },
    ],
    motion: {
      alt: 'Animation: a design comes together from three panels, changes colour, gets its texts and is exported.',
      title: 'Grand opening',
      subtitle: 'This Saturday',
      cta: 'Join us',
    },
  },
  features: {
    title: 'Everything you need to design',
    lead: 'Clear tools to create without the hassle, from a single post to a whole presentation.',
    items: [
      {
        icon: 'layout',
        title: 'Templates and sizes',
        text: '75 templates for Instagram, presentations, posters, invitations, documents and email. Search by design type or create one at your own size.',
      },
      {
        icon: 'shapes',
        title: 'Shapes, lines and quotes',
        text: '215 shapes: polygons, stars, arrows, diagrams, speech bubbles, clouds and quote marks. Lines that connect to shapes.',
      },
      {
        icon: 'text-aa',
        title: 'Text with style',
        text: '33 bundled font families, font pairings, curved text, text inside shapes and text effects.',
      },
      {
        icon: 'eyedropper',
        title: 'Colour in every detail',
        text: 'An eyedropper with a loupe, solid colours and gradients, your design colours and preset palettes.',
      },
      {
        icon: 'magic-wand',
        title: 'Photos and images',
        text: 'Adjustments, filters, shadows and crops with shapes. Remove the background, upscale or erase objects with models that run on your computer.',
      },
      {
        icon: 'images',
        title: 'Photos, icons and illustrations',
        text: 'Find free photos and illustrations from the Elements panel, and bring in your images, SVG or PDF by drag and drop.',
      },
      {
        icon: 'chart-bar',
        title: 'Tables, charts and QR',
        text: 'Editable tables (import Excel or CSV), charts with their data editor, and QR codes with colours and a logo.',
      },
      {
        icon: 'palette',
        title: 'Brand kits',
        text: 'Keep logos, colours, text styles and fonts in a kit and apply them to any design. Share it as a .gafkit file.',
      },
      {
        icon: 'paint-brush',
        title: 'Freehand drawing',
        text: '16 brushes with their own style and vector strokes you can move and edit like any element.',
      },
      {
        icon: 'stack',
        title: 'Layers and precision',
        text: 'Layers you drag, lock and hide; guides, snapping, rulers, align and distribute.',
      },
      {
        icon: 'export',
        title: 'Export the way you need',
        text: 'PNG, JPEG, WebP and AVIF; SVG; vector PDF with CMYK and a password; editable PowerPoint.',
      },
      {
        icon: 'package',
        title: 'Asset packages',
        text: 'Install .gafpack packages with illustrations, icons, shapes, frames and templates, and uninstall them whenever you like.',
      },
    ],
  },
  showcase: {
    title: 'See Gafira in action',
    items: [
      {
        title: 'A clear, direct editor',
        text: 'What you need is in sight: elements on the left, your design in the middle and its properties on the right.',
        points: ['Design colours and an eyedropper', 'Shapes with a click or a drag', 'Undo and redo at every step'],
        alt: 'The Gafira editor with the Elements panel open and the properties of a shape',
      },
      {
        title: 'Start from a template',
        text: 'Home gathers your recent designs and templates by type, with a search to find the one you need.',
        points: ['Social media, presentations and prints', 'Your own size when you need it', 'Everything can be changed'],
        alt: 'The Gafira Home screen with design types and Instagram templates',
      },
      {
        title: 'Export in the right format',
        text: 'See the preview, pick format, pages, scale and quality, and keep your settings for next time.',
        points: ['Image, SVG, PDF and PowerPoint', 'Transparent background', 'Exports in the background'],
        alt: 'The Gafira Export dialog with the design preview',
      },
    ],
  },
  local: {
    title: 'It runs on your computer',
    text: 'Gafira is a desktop app. Your projects are saved as .gaf files on your computer, and the AI image tools run locally.',
    points: [
      {
        icon: 'hard-drives',
        title: 'Your files',
        text: 'A .gaf file keeps the design, its images and its versions.',
      },
      {
        icon: 'shield-check',
        title: 'AI on your computer',
        text: 'Removing a background or upscaling does not upload your images to any server.',
      },
      {
        icon: 'clock-counter-clockwise',
        title: 'Versions and recovery',
        text: 'Save versions inside the project and get your work back if something closes.',
      },
    ],
  },
  download: {
    title: 'Download Gafira',
    lead: 'Pick your system. Downloads come from the official release on GitHub.',
    version: 'Version',
    published: 'Released on',
    recommended: 'For your computer',
    windows: {
      name: 'Windows',
      installer: 'Installer',
      installerNote: '.exe · 64-bit',
      zip: 'Portable version',
      zipNote: '.zip · no install',
      note: 'If Windows shows a SmartScreen warning, choose “More info” and then “Run anyway”.',
    },
    macos: {
      name: 'macOS',
      dmg: 'Disk image',
      dmgNote: '.dmg · Apple Silicon',
      note: 'It is not notarised yet: the first time, right-click Gafira and choose “Open”.',
    },
    linux: {
      name: 'Linux',
      deb: '.deb package',
      debNote: 'Ubuntu, Debian and derivatives',
      appImage: 'AppImage',
      appImageNote: 'Any x86-64 distribution',
      note: 'The AppImage needs execute permission: chmod +x Gafira-*.AppImage',
    },
    checksums: 'SHA-256 checksums',
    notes: 'Release notes',
    all: 'All releases',
  },
  cta: {
    title: 'Start creating',
    text: 'Download Gafira and turn your next idea into a design ready to share.',
    button: 'Download Gafira',
  },
  footer: {
    tagline: 'Bring your ideas to life.',
    product: 'Product',
    releases: 'Releases on GitHub',
    rights: 'All rights reserved.',
    language: 'Language',
  },
  notFound: {
    title: 'We could not find this page',
    text: 'The link may have changed. Go back home to carry on.',
    home: 'Go home',
  },
};

export const copy: Record<Locale, Copy> = { es, en };

/** The path of the home page in `locale`. */
export function homePath(locale: Locale): string {
  return locale === 'es' ? '/' : '/en/';
}

/** "24 de noviembre de 2026" / "November 24, 2026". */
export function longDate(iso: string, locale: Locale): string {
  return new Intl.DateTimeFormat(locale === 'es' ? 'es' : 'en-US', {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
    timeZone: 'UTC',
  }).format(new Date(iso));
}
