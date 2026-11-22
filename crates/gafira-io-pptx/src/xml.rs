//! The XML parts of the presentation (ECMA-376 PresentationML and
//! DrawingML): each slide from a page, and the fixed parts around them.

use std::fmt::Write as _;

use gafira_color::Color;
use gafira_doc::{
    BackgroundFit, Document, FrameNode, Gradient, GradientKind, LineCap, Node, NodeKind, Page,
    PageBackground, PatternKind, Placement, RichText, ShapeGeometry, ShapePath, ShapeStroke,
    TextAlign, VerticalAlign,
};
use gafira_doc::{PathCommand, timeline::AssetRef};
use gafira_ports::export::ExportError;

use crate::Media;

const NS: &str = r#"xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main""#;
const HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#;
const EMPTY_TREE: &str = r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>"#;

/// One slide: its XML and the pictures it points at, by relationship.
pub(crate) struct Slide {
    pub(crate) xml: String,
    /// `rIdN` (from 2) and the picture's file in `ppt/media/`.
    pub(crate) pictures: Vec<(String, String)>,
    /// Whether an element had no PowerPoint form (a light) and was left out.
    pub(crate) skipped: bool,
}

/// What a slide is written with.
struct Writer<'p, 'm, 's> {
    page: &'p Page,
    media: &'m mut Media<'s>,
    /// EMU (1/914400 in) per document px.
    k: f64,
    next_id: u32,
    pictures: Vec<(String, String)>,
    skipped: bool,
}

/// The EMU per px of `document`.
fn emu_per_px(document: &Document) -> f64 {
    914_400.0 / f64::from(document.dpi().get())
}

/// The slide size of the presentation: the page's, in EMU, within what
/// PowerPoint takes (1 in to 56 in).
pub(crate) fn slide_size(document: &Document, page: &Page) -> Result<(i64, i64), ExportError> {
    let px = page
        .size
        .to_px(document.dpi())
        .map_err(|_| ExportError::Io(std::io::Error::other("the page size is not finite")))?;
    let k = emu_per_px(document);
    let clamp = |v: f64| (v * k).round().clamp(914_400.0, 51_206_400.0) as i64;
    Ok((clamp(px.width()), clamp(px.height())))
}

/// The slide of `page`.
pub(crate) fn slide(document: &Document, page: &Page, media: &mut Media<'_>) -> Slide {
    let mut writer = Writer {
        page,
        media,
        k: emu_per_px(document),
        next_id: 2,
        pictures: Vec::new(),
        skipped: false,
    };
    let background = page
        .background
        .as_ref()
        .and_then(|bg| writer.background(bg))
        .unwrap_or_default();
    let mut tree = String::new();
    for id in &page.roots {
        if let Some(node) = page.node(*id) {
            writer.node(&mut tree, node);
        }
    }
    let xml = format!(
        "{HEAD}<p:sld {NS}><p:cSld>{background}<p:spTree>{EMPTY_TREE}{tree}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"
    );
    Slide {
        xml,
        pictures: writer.pictures,
        skipped: writer.skipped,
    }
}

impl Writer<'_, '_, '_> {
    fn emu(&self, v: f64) -> i64 {
        (v * self.k).round() as i64
    }

    fn id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// The relationship of the picture of `asset`, added once per slide.
    fn picture(&mut self, asset: AssetRef) -> Option<String> {
        let file = self.media.name(asset)?;
        if let Some((rid, _)) = self.pictures.iter().find(|(_, f)| *f == file) {
            return Some(rid.clone());
        }
        let rid = format!("rId{}", self.pictures.len() + 2);
        self.pictures.push((rid.clone(), file));
        Some(rid)
    }

    /// `<a:xfrm>` of `placement`: its box, turn and flips.
    fn xfrm(&self, p: &Placement) -> String {
        let mut attrs = String::new();
        let rot = (p.rotation_deg.rem_euclid(360.0) * 60_000.0).round() as i64;
        if rot != 0 {
            let _ = write!(attrs, " rot=\"{rot}\"");
        }
        if p.flip.x {
            attrs.push_str(" flipH=\"1\"");
        }
        if p.flip.y {
            attrs.push_str(" flipV=\"1\"");
        }
        format!(
            "<a:xfrm{attrs}><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm>",
            self.emu(p.x),
            self.emu(p.y),
            self.emu(p.w.abs()).max(1),
            self.emu(p.h.abs()).max(1)
        )
    }

    /// The geometry of a shape or frame of `w` × `h` px.
    fn geometry(&self, geometry: &ShapeGeometry, (w, h): (f64, f64), radius: f64) -> String {
        match geometry {
            ShapeGeometry::Rect if radius > 0.0 => {
                let adj = (radius / w.min(h).max(1e-9) * 100_000.0)
                    .round()
                    .clamp(0.0, 50_000.0);
                format!(
                    "<a:prstGeom prst=\"roundRect\"><a:avLst><a:gd name=\"adj\" fmla=\"val {adj}\"/></a:avLst></a:prstGeom>"
                )
            }
            ShapeGeometry::Rect => "<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>".to_owned(),
            ShapeGeometry::Ellipse => {
                "<a:prstGeom prst=\"ellipse\"><a:avLst/></a:prstGeom>".to_owned()
            }
            ShapeGeometry::Path(path) => self.cust_geom(path, (w, h)),
        }
    }

    /// A path of the unit box drawn in `w` × `h` px.
    fn cust_geom(&self, path: &ShapePath, (w, h): (f64, f64)) -> String {
        let (ew, eh) = (self.emu(w.abs()).max(1), self.emu(h.abs()).max(1));
        let pt = |[x, y]: [f64; 2]| {
            format!(
                "<a:pt x=\"{}\" y=\"{}\"/>",
                (x * ew as f64).round() as i64,
                (y * eh as f64).round() as i64
            )
        };
        let mut d = String::new();
        for command in &path.commands {
            let _ = match *command {
                PathCommand::MoveTo(p) => write!(d, "<a:moveTo>{}</a:moveTo>", pt(p)),
                PathCommand::LineTo(p) => write!(d, "<a:lnTo>{}</a:lnTo>", pt(p)),
                PathCommand::QuadTo(c, p) => {
                    write!(d, "<a:quadBezTo>{}{}</a:quadBezTo>", pt(c), pt(p))
                }
                PathCommand::CubicTo(a, b, p) => {
                    write!(
                        d,
                        "<a:cubicBezTo>{}{}{}</a:cubicBezTo>",
                        pt(a),
                        pt(b),
                        pt(p)
                    )
                }
                PathCommand::Close => write!(d, "<a:close/>"),
            };
        }
        format!(
            "<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l=\"0\" t=\"0\" r=\"r\" b=\"b\"/><a:pathLst><a:path w=\"{ew}\" h=\"{eh}\">{d}</a:path></a:pathLst></a:custGeom>"
        )
    }

    fn node(&mut self, out: &mut String, node: &Node) {
        if !node.base.visible {
            return;
        }
        let name = escape(node.base.name.as_deref().unwrap_or(""));
        let opacity = node.base.opacity.clamp(0.0, 1.0);
        let p = node.base.placement;
        match &node.kind {
            NodeKind::Group(_) => {
                let id = self.id();
                let (x, y, w, h) = (
                    self.emu(p.x),
                    self.emu(p.y),
                    self.emu(p.w.abs()).max(1),
                    self.emu(p.h.abs()).max(1),
                );
                let _ = write!(
                    out,
                    "<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"{id}\" name=\"{name}\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{w}\" cy=\"{h}\"/><a:chOff x=\"{x}\" y=\"{y}\"/><a:chExt cx=\"{w}\" cy=\"{h}\"/></a:xfrm></p:grpSpPr>"
                );
                for child in &node.children {
                    if let Some(child) = self.page.node(*child) {
                        self.node(out, child);
                    }
                }
                out.push_str("</p:grpSp>");
            }
            NodeKind::Shape(shape) => {
                // A light is a picture of its own (spec 065): no PowerPoint
                // shape draws it.
                if shape.effect.is_some() {
                    self.skipped = true;
                    return;
                }
                let id = self.id();
                let fill = match (&shape.gradient, shape.fill) {
                    (Some(gradient), _) => grad_fill(gradient, opacity),
                    (None, Some(color)) => solid_fill(color, opacity),
                    (None, None) => "<a:noFill/>".to_owned(),
                };
                let text = shape
                    .text
                    .as_ref()
                    .map(|text| {
                        let pad = self.emu(p.w.abs().min(p.h.abs()) / 10.0);
                        let anchor = match text.valign {
                            VerticalAlign::Top => "t",
                            VerticalAlign::Middle => "ctr",
                            VerticalAlign::Bottom => "b",
                        };
                        let body = format!(
                            "<a:bodyPr wrap=\"square\" lIns=\"{pad}\" tIns=\"{pad}\" rIns=\"{pad}\" bIns=\"{pad}\" anchor=\"{anchor}\" rtlCol=\"0\"><a:noAutofit/></a:bodyPr>"
                        );
                        self.text_body(&body, &text.content, text.align, (0.0, None, None, opacity))
                    })
                    .unwrap_or_default();
                let _ = write!(
                    out,
                    "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"{name}\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr>{}{}{fill}{}</p:spPr>{text}</p:sp>",
                    self.xfrm(&p),
                    self.geometry(&shape.geometry, (p.w, p.h), shape.corner_radius),
                    self.line(shape.stroke, opacity)
                );
            }
            NodeKind::Text(text) => {
                let id = self.id();
                let warp = match text.curve {
                    c if c > 0.0 => "<a:prstTxWarp prst=\"textArchUp\"><a:avLst/></a:prstTxWarp>",
                    c if c < 0.0 => "<a:prstTxWarp prst=\"textArchDown\"><a:avLst/></a:prstTxWarp>",
                    _ => "",
                };
                let body = format!(
                    "<a:bodyPr wrap=\"square\" lIns=\"0\" tIns=\"0\" rIns=\"0\" bIns=\"0\" anchor=\"t\" rtlCol=\"0\">{warp}<a:noAutofit/></a:bodyPr>"
                );
                let fill = text.gradient.as_ref().map(|g| grad_fill(g, opacity));
                let body = self.text_body(
                    &body,
                    &text.content,
                    text.align,
                    (
                        text.letter_spacing,
                        text.line_height,
                        fill.as_deref(),
                        opacity,
                    ),
                );
                let _ = write!(
                    out,
                    "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"{name}\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr><p:spPr>{}<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:noFill/></p:spPr>{body}</p:sp>",
                    self.xfrm(&p)
                );
            }
            NodeKind::Image(image) => {
                let Some(rid) = self.picture(image.asset) else {
                    self.skipped = true;
                    return;
                };
                let id = self.id();
                let _ = write!(
                    out,
                    "<p:pic><p:nvPicPr><p:cNvPr id=\"{id}\" name=\"{name}\"/><p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed=\"{rid}\">{}</a:blip><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr>{}<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr></p:pic>",
                    alpha_mod(opacity),
                    self.xfrm(&p)
                );
            }
            NodeKind::Frame(frame) => self.frame(out, frame, (&p, &name, opacity)),
            // A QR code goes as its shapes and its logo (spec 080); one that
            // cannot be made draws nothing anywhere.
            NodeKind::Qr(qr) => {
                for part in gafira_qr::parts(qr, &p).unwrap_or_default() {
                    let mut base = node.base.clone();
                    base.placement = part.placement;
                    self.node(out, &Node::new(base, part.kind));
                }
            }
            NodeKind::Table(table) => self.table(out, table, (&p, &name, opacity)),
            // A plug-in's element has no PowerPoint form.
            NodeKind::Extension(_) => self.skipped = true,
            NodeKind::Line(line) => {
                let [a, b] = line.ends(self.page, &p);
                let id = self.id();
                let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
                let (w, h) = ((b.x - a.x).abs(), (b.y - a.y).abs());
                let mut flips = String::new();
                if b.x < a.x {
                    flips.push_str(" flipH=\"1\"");
                }
                if b.y < a.y {
                    flips.push_str(" flipV=\"1\"");
                }
                let end = |cap: LineCap, tag: &str| match cap {
                    LineCap::Arrow => format!("<a:{tag} type=\"triangle\"/>"),
                    LineCap::Circle => format!("<a:{tag} type=\"oval\"/>"),
                    LineCap::Diamond | LineCap::DiamondFilled => {
                        format!("<a:{tag} type=\"diamond\"/>")
                    }
                    _ => String::new(),
                };
                let _ = write!(
                    out,
                    "<p:cxnSp><p:nvCxnSpPr><p:cNvPr id=\"{id}\" name=\"{name}\"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr><a:xfrm{flips}><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm><a:prstGeom prst=\"line\"><a:avLst/></a:prstGeom><a:ln w=\"{}\">{}{}{}</a:ln></p:spPr></p:cxnSp>",
                    self.emu(x0),
                    self.emu(y0),
                    self.emu(w),
                    self.emu(h),
                    self.emu(line.width).max(1),
                    solid_fill(line.color, opacity),
                    end(line.start.cap, "headEnd"),
                    end(line.end.cap, "tailEnd")
                );
            }
        }
    }

    /// A frame: its photo as a picture cut by its shape and cropped as the
    /// frame shows it, or, empty, its outline.
    fn frame(
        &mut self,
        out: &mut String,
        frame: &FrameNode,
        (p, name, opacity): (&Placement, &str, f32),
    ) {
        let (w, h) = (p.w.abs(), p.h.abs());
        let geometry = self.geometry(&frame.clip_shape, (w, h), frame.corner_radius);
        let line = self.line(frame.stroke, opacity);
        let photo = frame.content.and_then(|content| {
            let rid = self.picture(content.asset)?;
            let bounds = frame.photo_bounds(w, h)?;
            Some((rid, bounds))
        });
        let id = self.id();
        match photo {
            Some((rid, b)) => {
                // What of the photo shows: the frame's box over the photo's.
                let share = |v: f64, size: f64| (v / size.max(1e-9) * 100_000.0).round() as i64;
                let (bw, bh) = (b.width(), b.height());
                let crop = format!(
                    "<a:srcRect l=\"{}\" t=\"{}\" r=\"{}\" b=\"{}\"/>",
                    share(-b.x0, bw),
                    share(-b.y0, bh),
                    share(b.x1 - w, bw),
                    share(b.y1 - h, bh)
                );
                let _ = write!(
                    out,
                    "<p:pic><p:nvPicPr><p:cNvPr id=\"{id}\" name=\"{name}\"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed=\"{rid}\">{}</a:blip>{crop}<a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr>{}{geometry}{line}</p:spPr></p:pic>",
                    alpha_mod(opacity),
                    self.xfrm(p)
                );
            }
            None => {
                let _ = write!(
                    out,
                    "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"{name}\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr>{}{geometry}<a:noFill/>{line}</p:spPr></p:sp>",
                    self.xfrm(p)
                );
            }
        }
    }

    /// A table as a PowerPoint table (S81-13): its grid, its merges, its
    /// fills, borders and words.
    fn table(
        &mut self,
        out: &mut String,
        table: &gafira_doc::TableNode,
        (p, name, opacity): (&Placement, &str, f32),
    ) {
        let id = self.id();
        let layout = gafira_table::Layout::of(table, (p.w.abs(), p.h.abs()));
        let _ = write!(
            out,
            "<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"{id}\" name=\"{name}\"/><p:cNvGraphicFramePr><a:graphicFrameLocks noGrp=\"1\"/></p:cNvGraphicFramePr><p:nvPr/></p:nvGraphicFramePr>{}<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/table\"><a:tbl><a:tblPr{}/><a:tblGrid>",
            self.xfrm(p)
                .replacen("<a:xfrm", "<p:xfrm", 1)
                .replace("</a:xfrm>", "</p:xfrm>"),
            if table.header { " firstRow=\"1\"" } else { "" }
        );
        for (x0, x1) in &layout.columns {
            let _ = write!(out, "<a:gridCol w=\"{}\"/>", self.emu(x1 - x0).max(1));
        }
        out.push_str("</a:tblGrid>");
        for (r, (y0, y1)) in layout.rows.iter().enumerate() {
            let _ = write!(out, "<a:tr h=\"{}\">", self.emu(y1 - y0).max(1));
            for c in 0..layout.columns.len() {
                self.table_cell(out, table, (r, c), opacity);
            }
            out.push_str("</a:tr>");
        }
        out.push_str("</a:tbl></a:graphicData></a:graphic></p:graphicFrame>");
    }

    fn table_cell(
        &self,
        out: &mut String,
        table: &gafira_doc::TableNode,
        (r, c): (usize, usize),
        opacity: f32,
    ) {
        let Some(cell) = table.cell(r, c) else { return };
        let style = &cell.style;
        let mut spans = String::new();
        if let Some(m) = table.merge_at(r, c) {
            if (m.row, m.col) == (r, c) {
                if m.cols > 1 {
                    let _ = write!(spans, " gridSpan=\"{}\"", m.cols);
                }
                if m.rows > 1 {
                    let _ = write!(spans, " rowSpan=\"{}\"", m.rows);
                }
            } else {
                if c > m.col {
                    spans.push_str(" hMerge=\"1\"");
                }
                if r > m.row {
                    spans.push_str(" vMerge=\"1\"");
                }
            }
        }
        let content = gafira_doc::RichText::plain(
            &cell.text,
            &gafira_doc::SpanStyle {
                font: style.font.clone(),
                size: style.size,
                color: style.color,
                bold: style.bold,
                italic: style.italic,
            },
        );
        let body = self
            .text_body(
                "<a:bodyPr/>",
                &content,
                style.align,
                (0.0, None, None, opacity),
            )
            .replacen("<p:txBody>", "<a:txBody>", 1)
            .replace("</p:txBody>", "</a:txBody>");
        let anchor = match style.valign {
            VerticalAlign::Top => "t",
            VerticalAlign::Middle => "ctr",
            VerticalAlign::Bottom => "b",
        };
        let pad = self.emu(table.padding);
        let _ = write!(
            out,
            "<a:tc{spans}>{body}<a:tcPr marL=\"{pad}\" marR=\"{pad}\" marT=\"{pad}\" marB=\"{pad}\" anchor=\"{anchor}\">"
        );
        let b = style.borders;
        // The edge two cells share may be kept by either of them.
        let next =
            |r: Option<usize>, c: Option<usize>| table.cell(r?, c?).map(|cell| cell.style.borders);
        let shared = table.spacing == 0.0;
        for (tag, own, other) in [
            (
                "lnL",
                b.left,
                next(Some(r), c.checked_sub(1)).and_then(|o| o.right),
            ),
            (
                "lnR",
                b.right,
                next(Some(r), Some(c + 1)).and_then(|o| o.left),
            ),
            (
                "lnT",
                b.top,
                next(r.checked_sub(1), Some(c)).and_then(|o| o.bottom),
            ),
            (
                "lnB",
                b.bottom,
                next(Some(r + 1), Some(c)).and_then(|o| o.top),
            ),
        ] {
            let stroke = own.or(other.filter(|_| shared)).filter(|s| s.width > 0.0);
            let _ = match stroke {
                Some(stroke) => write!(
                    out,
                    "<a:{tag} w=\"{}\">{}</a:{tag}>",
                    self.emu(stroke.width).max(1),
                    solid_fill(stroke.color, opacity)
                ),
                None => write!(out, "<a:{tag} w=\"0\"><a:noFill/></a:{tag}>"),
            };
        }
        match style.fill {
            Some(color) => out.push_str(&solid_fill(color, opacity)),
            None => out.push_str("<a:noFill/>"),
        }
        out.push_str("</a:tcPr></a:tc>");
    }

    /// `<a:ln>` of an outline, or none.
    fn line(&self, stroke: Option<ShapeStroke>, opacity: f32) -> String {
        match stroke.filter(|s| s.width > 0.0) {
            Some(stroke) => format!(
                "<a:ln w=\"{}\">{}</a:ln>",
                self.emu(stroke.width).max(1),
                solid_fill(stroke.color, opacity)
            ),
            None => "<a:ln><a:noFill/></a:ln>".to_owned(),
        }
    }

    /// A text body: one paragraph per paragraph of `content`, one run per
    /// span; `(letter spacing, line height, gradient fill, opacity)` of the
    /// text.
    fn text_body(
        &self,
        body_pr: &str,
        content: &RichText,
        align: TextAlign,
        (spacing, line_height, gradient, opacity): (f64, Option<f64>, Option<&str>, f32),
    ) -> String {
        let algn = match align {
            TextAlign::Left => "l",
            TextAlign::Center => "ctr",
            TextAlign::Right => "r",
            TextAlign::Justify => "just",
        };
        let pt_per_px = 72.0 * self.k / 914_400.0;
        let ln_spc = line_height
            .map(|h| {
                format!(
                    "<a:lnSpc><a:spcPct val=\"{}\"/></a:lnSpc>",
                    (h * 100_000.0).round() as i64
                )
            })
            .unwrap_or_default();
        let mut paragraphs = String::new();
        for paragraph in &content.paragraphs {
            let mut runs = String::new();
            let mut last = 1800;
            for span in &paragraph.spans {
                let style = &span.style;
                let size = (style.size * pt_per_px * 100.0)
                    .round()
                    .clamp(100.0, 400_000.0) as i64;
                last = size;
                let spc = (spacing / 1000.0 * style.size * pt_per_px * 100.0).round() as i64;
                let fill = gradient
                    .map(str::to_owned)
                    .unwrap_or_else(|| solid_fill(style.color, opacity));
                let font = escape(&style.font);
                let _ = write!(
                    runs,
                    "<a:r><a:rPr lang=\"es-ES\" sz=\"{size}\" b=\"{}\" i=\"{}\"{} dirty=\"0\">{fill}<a:latin typeface=\"{font}\"/><a:cs typeface=\"{font}\"/></a:rPr><a:t>{}</a:t></a:r>",
                    u8::from(style.bold),
                    u8::from(style.italic),
                    if spc == 0 {
                        String::new()
                    } else {
                        format!(" spc=\"{spc}\"")
                    },
                    escape(&span.text)
                );
            }
            let _ = write!(
                paragraphs,
                "<a:p><a:pPr algn=\"{algn}\">{ln_spc}</a:pPr>{runs}<a:endParaRPr lang=\"es-ES\" sz=\"{last}\" dirty=\"0\"/></a:p>"
            );
        }
        if paragraphs.is_empty() {
            paragraphs.push_str("<a:p><a:endParaRPr lang=\"es-ES\" dirty=\"0\"/></a:p>");
        }
        format!("<p:txBody>{body_pr}<a:lstStyle/>{paragraphs}</p:txBody>")
    }

    /// The background of the slide.
    fn background(&mut self, background: &PageBackground) -> Option<String> {
        let fill = match background {
            PageBackground::Solid(color) => solid_fill(*color, 1.0),
            PageBackground::Gradient(gradient) => grad_fill(gradient, 1.0),
            // PowerPoint's own patterns, in the same two colours.
            PageBackground::Pattern { kind, color, back } => format!(
                "<a:pattFill prst=\"{}\"><a:fgClr>{}</a:fgClr><a:bgClr>{}</a:bgClr></a:pattFill>",
                pattern_preset(*kind),
                srgb(*color, 1.0),
                srgb(*back, 1.0)
            ),
            PageBackground::Image { asset, fit, .. } => {
                let rid = self.picture(*asset)?;
                let mode = match fit {
                    BackgroundFit::Tile => {
                        "<a:tile tx=\"0\" ty=\"0\" sx=\"100000\" sy=\"100000\" flip=\"none\" algn=\"tl\"/>"
                    }
                    _ => "<a:stretch><a:fillRect/></a:stretch>",
                };
                format!(
                    "<a:blipFill dpi=\"0\" rotWithShape=\"1\"><a:blip r:embed=\"{rid}\"/>{mode}</a:blipFill>"
                )
            }
        };
        Some(format!(
            "<p:bg><p:bgPr>{fill}<a:effectLst/></p:bgPr></p:bg>"
        ))
    }
}

/// The PowerPoint pattern nearest to a Gafira one.
fn pattern_preset(kind: PatternKind) -> &'static str {
    match kind {
        PatternKind::Dots => "dotGrid",
        PatternKind::Grid | PatternKind::GridFine => "smGrid",
        PatternKind::Stripes => "wdUpDiag",
        PatternKind::Checks => "smCheck",
        PatternKind::Zigzag => "zigZag",
        PatternKind::Crosses => "cross",
        PatternKind::Bricks | PatternKind::BricksLarge => "horzBrick",
        PatternKind::GridDiagonal => "diagCross",
        PatternKind::Hexagons => "shingle",
        PatternKind::Isometric | PatternKind::Triangles => "trellis",
        PatternKind::Diamonds => "openDmnd",
    }
}

/// The straight sRGB channels of `color`.
fn srgb8(color: Color) -> [u8; 4] {
    let Color::Srgb(c) = color.convert(gafira_color::ColorSpaceId::Srgb) else {
        return [0, 0, 0, 255];
    };
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [q(c.r), q(c.g), q(c.b), q(c.a)]
}

/// `<a:srgbClr>` of `color`, its alpha times `opacity`.
fn srgb(color: Color, opacity: f32) -> String {
    let [r, g, b, a] = srgb8(color);
    let alpha = (f32::from(a) / 255.0 * opacity).clamp(0.0, 1.0);
    if alpha < 1.0 {
        format!(
            "<a:srgbClr val=\"{r:02X}{g:02X}{b:02X}\"><a:alpha val=\"{}\"/></a:srgbClr>",
            (alpha * 100_000.0).round() as i64
        )
    } else {
        format!("<a:srgbClr val=\"{r:02X}{g:02X}{b:02X}\"/>")
    }
}

fn solid_fill(color: Color, opacity: f32) -> String {
    format!("<a:solidFill>{}</a:solidFill>", srgb(color, opacity))
}

/// A gradient: its stops and its direction (0° left to right, clockwise),
/// or round from the middle.
fn grad_fill(gradient: &Gradient, opacity: f32) -> String {
    let mut stops = String::new();
    for stop in &gradient.stops {
        let _ = write!(
            stops,
            "<a:gs pos=\"{}\">{}</a:gs>",
            (f64::from(stop.offset.clamp(0.0, 1.0)) * 100_000.0).round() as i64,
            srgb(stop.color, opacity)
        );
    }
    let shade = match gradient.kind {
        GradientKind::Linear { angle_deg } => format!(
            "<a:lin ang=\"{}\" scaled=\"0\"/>",
            (angle_deg.rem_euclid(360.0) * 60_000.0).round() as i64
        ),
        GradientKind::Radial => "<a:path path=\"circle\"><a:fillToRect l=\"50000\" t=\"50000\" r=\"50000\" b=\"50000\"/></a:path>".to_owned(),
    };
    format!("<a:gradFill rotWithShape=\"1\"><a:gsLst>{stops}</a:gsLst>{shade}</a:gradFill>")
}

/// A picture's opacity, when it is not whole.
fn alpha_mod(opacity: f32) -> String {
    if opacity < 1.0 {
        format!(
            "<a:alphaModFix amt=\"{}\"/>",
            (opacity * 100_000.0).round() as i64
        )
    } else {
        String::new()
    }
}

/// Text escaped for XML.
pub(crate) fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// `[Content_Types].xml`.
pub(crate) fn content_types(slides: usize, media: &Media<'_>) -> String {
    let mut overrides = String::new();
    for n in 1..=slides {
        let _ = write!(
            overrides,
            "<Override PartName=\"/ppt/slides/slide{n}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>"
        );
    }
    let jpeg = if media.files.iter().any(|(name, _)| name.ends_with(".jpeg")) {
        "<Default Extension=\"jpeg\" ContentType=\"image/jpeg\"/>"
    } else {
        ""
    };
    format!(
        "{HEAD}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"png\" ContentType=\"image/png\"/>{jpeg}<Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/><Override PartName=\"/ppt/slideMasters/slideMaster1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml\"/><Override PartName=\"/ppt/slideLayouts/slideLayout1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml\"/><Override PartName=\"/ppt/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/><Override PartName=\"/ppt/presProps.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presProps+xml\"/><Override PartName=\"/ppt/viewProps.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.viewProps+xml\"/><Override PartName=\"/ppt/tableStyles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml\"/><Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/><Override PartName=\"/docProps/app.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.extended-properties+xml\"/>{overrides}</Types>"
    )
}

pub(crate) const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#;

pub(crate) const CORE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title></dc:title><dc:creator>Gafira</dc:creator></cp:coreProperties>"#;

pub(crate) fn app(slides: usize) -> String {
    format!(
        "{HEAD}<Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\"><Application>Gafira</Application><Slides>{slides}</Slides></Properties>"
    )
}

pub(crate) fn presentation(slides: usize, (cx, cy): (i64, i64)) -> String {
    let mut ids = String::new();
    for n in 0..slides {
        let _ = write!(ids, "<p:sldId id=\"{}\" r:id=\"rId{}\"/>", 256 + n, n + 2);
    }
    format!(
        "{HEAD}<p:presentation {NS} saveSubsetFonts=\"1\"><p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rId1\"/></p:sldMasterIdLst><p:sldIdLst>{ids}</p:sldIdLst><p:sldSz cx=\"{cx}\" cy=\"{cy}\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/></p:presentation>"
    )
}

pub(crate) fn presentation_rels(slides: usize) -> String {
    let mut rels = String::from(
        "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster\" Target=\"slideMasters/slideMaster1.xml\"/>",
    );
    for n in 1..=slides {
        let _ = write!(
            rels,
            "<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{n}.xml\"/>",
            n + 1
        );
    }
    let base = slides + 2;
    for (k, (kind, target)) in [
        ("presProps", "presProps.xml"),
        ("viewProps", "viewProps.xml"),
        ("theme", "theme/theme1.xml"),
        ("tableStyles", "tableStyles.xml"),
    ]
    .into_iter()
    .enumerate()
    {
        let _ = write!(
            rels,
            "<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}\" Target=\"{target}\"/>",
            base + k
        );
    }
    format!(
        "{HEAD}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{rels}</Relationships>"
    )
}

pub(crate) fn slide_rels(pictures: &[(String, String)]) -> String {
    let mut rels = String::from(
        "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/>",
    );
    for (rid, file) in pictures {
        let _ = write!(
            rels,
            "<Relationship Id=\"{rid}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"../media/{file}\"/>"
        );
    }
    format!(
        "{HEAD}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{rels}</Relationships>"
    )
}

pub(crate) const PRES_PROPS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentationPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"/>"#;

pub(crate) const VIEW_PROPS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:viewPr xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:normalViewPr><p:restoredLeft sz="15620"/><p:restoredTop sz="94660"/></p:normalViewPr><p:gridSpacing cx="76200" cy="76200"/></p:viewPr>"#;

pub(crate) const TABLE_STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" def="{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}"/>"#;

pub(crate) const MASTER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:bg><p:bgRef idx="1001"><a:schemeClr val="bg1"/></p:bgRef></p:bg><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld><p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/><p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst><p:txStyles><p:titleStyle><a:lvl1pPr><a:defRPr sz="4400"/></a:lvl1pPr></p:titleStyle><p:bodyStyle><a:lvl1pPr><a:defRPr sz="1800"/></a:lvl1pPr></p:bodyStyle><p:otherStyle><a:lvl1pPr><a:defRPr sz="1800"/></a:lvl1pPr></p:otherStyle></p:txStyles></p:sldMaster>"#;

pub(crate) const MASTER_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/></Relationships>"#;

pub(crate) const LAYOUT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank" preserve="1"><p:cSld name="Blank"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"#;

pub(crate) const LAYOUT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/></Relationships>"#;

pub(crate) const THEME: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Gafira"><a:themeElements><a:clrScheme name="Gafira"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="18243B"/></a:dk2><a:lt2><a:srgbClr val="F4F6F8"/></a:lt2><a:accent1><a:srgbClr val="3FBFA9"/></a:accent1><a:accent2><a:srgbClr val="FF6B5A"/></a:accent2><a:accent3><a:srgbClr val="7B61FF"/></a:accent3><a:accent4><a:srgbClr val="F4A21B"/></a:accent4><a:accent5><a:srgbClr val="2563EB"/></a:accent5><a:accent6><a:srgbClr val="E84545"/></a:accent6><a:hlink><a:srgbClr val="2563EB"/></a:hlink><a:folHlink><a:srgbClr val="7B61FF"/></a:folHlink></a:clrScheme><a:fontScheme name="Gafira"><a:majorFont><a:latin typeface="Inter"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Inter"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme><a:fmtScheme name="Gafira"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>"#;
