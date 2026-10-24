# Generador de PDF del manual Gafira

Este kit recupera el Markdown v2, los SVG del logotipo y el HTML conservado de v1. El **script Python ha sido reconstruido**: no es una copia del código ejecutado originalmente. Los PDF anteriores indican que su productor fue **WeasyPrint 68.0**.

## Instalación

```bash
python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate
pip install -r requirements.txt
```

En Linux, WeasyPrint puede requerir librerías del sistema como Pango/Cairo y fuentes instaladas.

## Generar el manual actualizado v2

```bash
python generar_manual.py
# genera Gafira_Manual_de_Marca.pdf
```

## Personalizar

- `manual-gafira.md`: contenido del manual.
- `manual.css`: estilos de impresión, portada, títulos, tablas, márgenes.
- `assets/*.svg`: gráficos de marca.
- `generar_manual.py`: conversión Markdown -> HTML -> PDF.

Ejemplo:

```bash
python generar_manual.py --input manual-gafira.md --output manual-nuevo.pdf
```

Para probar el HTML recuperado de v1 (con **estilos CSS reconstruidos**, no exactamente los originales):

```bash
python generar_manual.py --html gafira-manual-de-marca-v2.html --output manual-v1-recreado.pdf
```

> El resultado de este generador es una **reproducción funcional** del flujo de producción. No se promete identidad visual píxel por píxel con el PDF v2 entregado anteriormente.
