//! Raster PDF (the compatibility option): a hand-written PDF 1.4 where each page is rendered by
//! `genha-render` and embedded as one JPEG (`DCTDecode`), with an invisible text layer (text
//! render mode 3, standard Helvetica) so the text can still be searched, selected and copied.

use std::fmt::Write as _;

use genha_doc::{Document, ElementKind, Page};
use genha_geom::Point;
use genha_render::{RenderOptions, Renderer, element_transform, encode_jpeg};
use genha_text::FontBook;

use crate::{Geometry, PT_PER_PX, PdfError, PdfOptions, crop_mark_lines, fitting_scale, num, nums, pdf_string, with_bleed};

/// Writes the raster PDF of `pages` (indices checked by the caller).
pub(crate) fn export(doc: &Document, fonts: &FontBook, opts: &PdfOptions, pages: &[usize]) -> Result<Vec<u8>, PdfError> {
    let dpi = opts.resolution();
    let mut renderer = Renderer::new();
    let mut pdf = Pdf::new();
    // Objects 1–4: catalog, page tree, info, the text layer's font; then three per page.
    let n = pages.len();
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 5 + i * 3)).collect();
    pdf.object(1, b"<< /Type /Catalog /Pages 2 0 R >>");
    pdf.object(2, format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).as_bytes());
    pdf.object(3, format!("<< /Title {} /Producer (GenHa) /Creator (GenHa) >>", pdf_string(&doc.title)).as_bytes());
    pdf.object(4, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
    for (k, &index) in pages.iter().enumerate() {
        let page = doc.page(index).map_err(|_| PdfError::NoPage(index))?;
        let g = Geometry::new(page, opts);
        // With a bleed, render a copy of the page grown by the bleed on every side.
        let raster = if g.bleed > 0.0 {
            let mut big = Document { pages: vec![with_bleed(page, g.bleed)], versions: vec![], ..Document::new(&doc.title, 1.0, 1.0) };
            big.assets = doc.assets.clone();
            big.brand = doc.brand.clone();
            let scale = fitting_scale(g.page_w, g.page_h, dpi / 96.0);
            renderer.render_page(&big, 0, fonts, &RenderOptions { scale, transparent_background: false })?
        } else {
            let scale = fitting_scale(page.width, page.height, dpi / 96.0);
            renderer.render_page(doc, index, fonts, &RenderOptions { scale, transparent_background: false })?
        };
        let jpeg = encode_jpeg(&raster, opts.jpeg_quality.clamp(1, 100))?;
        let (page_obj, content_obj, image_obj) = (5 + k * 3, 6 + k * 3, 7 + k * 3);
        let [mx, my] = [g.media_w * PT_PER_PX, g.media_h * PT_PER_PX];
        // PDF boxes are y-up; trim and bleed are centred in the media box, so they read the same.
        let trim = g.trim.map(|v| v * PT_PER_PX);
        let bleed_box = g.bleed_box.map(|v| v * PT_PER_PX);
        let boxes = if g.has_boxes() { format!(" /TrimBox [{}] /BleedBox [{}]", nums(&trim), nums(&bleed_box)) } else { String::new() };
        pdf.object(
            page_obj,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}]{boxes} /Resources << /XObject << /Im0 {image_obj} 0 R >> /Font << /F1 4 0 R >> /ProcSet [/PDF /Text /ImageC] >> /Contents {content_obj} 0 R >>",
                num(mx),
                num(my)
            )
            .as_bytes(),
        );
        let slug = g.slug * PT_PER_PX;
        let mut content = format!("q\n{} 0 0 {} {} {} cm\n/Im0 Do\nQ\n", num(g.page_w * PT_PER_PX), num(g.page_h * PT_PER_PX), num(slug), num(slug));
        if opts.crop_marks {
            content.push_str("q\n0 G 0.3 w\n");
            for (a, b) in crop_mark_lines(&g) {
                // Page pixels, y down → points, y up.
                let _ = writeln!(
                    content,
                    "{} {} m {} {} l S",
                    num(a.x * PT_PER_PX),
                    num(my - a.y * PT_PER_PX),
                    num(b.x * PT_PER_PX),
                    num(my - b.y * PT_PER_PX)
                );
            }
            content.push_str("Q\n");
        }
        if opts.text_layer {
            content.push_str(&text_layer(page, fonts, trim[0], trim[3]));
        }
        pdf.stream(content_obj, &format!("<< /Length {} >>", content.len()), content.as_bytes());
        let dict = format!(
            "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>",
            raster.width,
            raster.height,
            jpeg.len()
        );
        pdf.stream(image_obj, &dict, &jpeg);
    }
    Ok(pdf.finish(1, 3))
}

/// Invisible text (render mode 3) where the page's text is, for search, selection and copying.
/// `x0` is the trim box's left edge and `top` its top edge, in points.
fn text_layer(page: &Page, fonts: &FontBook, x0: f64, top: f64) -> String {
    let mut out = String::new();
    page.for_each(&mut |e| {
        if e.hidden {
            return;
        }
        let ElementKind::Text(t) = &e.kind else { return };
        let size = t.style.font_size;
        if !size.is_finite() || size <= 0.0 {
            return;
        }
        let m = element_transform(e);
        let layout = genha_text::layout(fonts, &t.text, &t.style, e.frame.width);
        // Page pixels (y down) to points (y up).
        let to_pt = |p: Point| (x0 + p.x * PT_PER_PX, top - p.y * PT_PER_PX);
        let c = m.as_coeffs();
        for line in &layout.lines {
            let text = line.text.trim_end();
            if text.is_empty() {
                continue;
            }
            let (ox, oy) = to_pt(m * Point::new(line.x, line.baseline));
            // Unit vectors of the line's x axis and up axis in points, scaled to 1 font unit.
            let (ax, ay) = (c[0] * PT_PER_PX, -c[1] * PT_PER_PX);
            let (bx, by) = (-c[2] * PT_PER_PX, c[3] * PT_PER_PX);
            let (enc, count) = win_ansi(text);
            // Stretch Helvetica's (approximate) width to the line's real width.
            let natural = count as f64 * 0.52 * size;
            let tz = if natural > 0.0 && line.width > 0.0 { (line.width / natural * 100.0).clamp(10.0, 1000.0) } else { 100.0 };
            let _ = writeln!(
                out,
                "BT 3 Tr /F1 {} Tf {} Tz {} {} {} {} {} {} Tm {enc} Tj ET",
                num(size),
                num(tz),
                num(ax),
                num(ay),
                num(bx),
                num(by),
                num(ox),
                num(oy)
            );
        }
    });
    out
}

/// `s` as a PDF string in WinAnsi encoding (characters outside it become `?`), and its length.
pub(crate) fn win_ansi(s: &str) -> (String, usize) {
    let mut out = String::from("(");
    let mut n = 0;
    for ch in s.chars() {
        n += 1;
        let b: u8 = match ch {
            '(' | ')' | '\\' => {
                out.push('\\');
                out.push(ch);
                continue;
            }
            c if (' '..='~').contains(&c) => {
                out.push(c);
                continue;
            }
            c if ('\u{a0}'..='\u{ff}').contains(&c) => c as u32 as u8,
            '€' => 0x80,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '•' => 0x95,
            '–' => 0x96,
            '—' => 0x97,
            '…' => 0x85,
            _ => b'?',
        };
        let _ = write!(out, "\\{b:03o}");
    }
    out.push(')');
    (out, n)
}

/// A PDF being written: the bytes so far and where each object starts.
struct Pdf {
    out: Vec<u8>,
    /// Byte offset of object `i` at index `i - 1`.
    offsets: Vec<usize>,
}

impl Pdf {
    fn new() -> Self {
        let mut out = b"%PDF-1.4\n".to_vec();
        // A comment with bytes above 127 marks the file as binary for transfer tools.
        out.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");
        Pdf { out, offsets: vec![] }
    }

    fn mark(&mut self, id: usize) {
        if self.offsets.len() < id {
            self.offsets.resize(id, 0);
        }
        if let Some(slot) = id.checked_sub(1).and_then(|i| self.offsets.get_mut(i)) {
            *slot = self.out.len();
        }
        self.out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
    }

    fn object(&mut self, id: usize, body: &[u8]) {
        self.mark(id);
        self.out.extend_from_slice(body);
        self.out.extend_from_slice(b"\nendobj\n");
    }

    fn stream(&mut self, id: usize, dict: &str, data: &[u8]) {
        self.mark(id);
        self.out.extend_from_slice(dict.as_bytes());
        self.out.extend_from_slice(b"\nstream\n");
        self.out.extend_from_slice(data);
        self.out.extend_from_slice(b"\nendstream\nendobj\n");
    }

    fn finish(mut self, root: usize, info: usize) -> Vec<u8> {
        let xref = self.out.len();
        let size = self.offsets.len() + 1;
        let mut t = format!("xref\n0 {size}\n0000000000 65535 f \n");
        for off in &self.offsets {
            let _ = writeln!(t, "{off:010} 00000 n ");
        }
        let _ = write!(t, "trailer\n<< /Size {size} /Root {root} 0 R /Info {info} 0 R >>\nstartxref\n{xref}\n%%EOF\n");
        self.out.extend_from_slice(t.as_bytes());
        self.out
    }
}
