//! Spike aislado: un párrafo → layout → `Draw` (display list) mapeable a paint ops Kai.
//!
//! No usa `wordcraft-ui-egui` ni llama a `wordcraft-render` / vello. `wordcraft-engine`
//! arrastra `wordcraft-render` (vello_cpu) y `wordcraft-pdf` como dependencias transitivas;
//! este binario no las invoca. El mapeo a paint usa solo `wordcraft_layout::display`.

use wordcraft_doc::{Document, StoryRef};
use wordcraft_engine::Session;
use wordcraft_fonts::FontDb;
use wordcraft_layout::display::{page_display, DisplayOptions, Draw};
use wordcraft_layout::{layout, LayoutCache, LayoutOptions};

fn main() {
    let _db = FontDb::global();
    let doc = Document::from_text("Hola Kai — un párrafo mínimo para el spike Document.");
    let session = Session::new(doc.clone());
    assert!(!session.doc.plain_text(StoryRef::Body).is_empty());

    let mut cache = LayoutCache::new();
    let lay = layout(&doc, &mut cache, &LayoutOptions::default());
    assert!(!lay.pages.is_empty(), "layout debe producir al menos una página");

    let page = &lay.pages[0];
    let draws = page_display(&doc, page, &DisplayOptions::default());

    println!("spike-ok");
    println!("pages={}", lay.pages.len());
    println!("page_size_pt={:.1}x{:.1}", page.w, page.h);
    println!("draw_items={}", draws.len());
    println!("draw_summary={}", summarize_draws(&draws));
    println!("sample={}", sample_line(&draws));
}

fn summarize_draws(draws: &[Draw]) -> String {
    let mut glyphs = 0usize;
    let mut fills = 0usize;
    let mut lines = 0usize;
    let mut other = 0usize;
    walk(draws, &mut |d| match d {
        Draw::Glyphs { .. } => glyphs += 1,
        Draw::Fill { .. } => fills += 1,
        Draw::Line { .. } => lines += 1,
        _ => other += 1,
    });
    format!("glyphs={glyphs} fills={fills} lines={lines} other={other}")
}

fn sample_line(draws: &[Draw]) -> String {
    let mut out = None;
    walk(draws, &mut |d| {
        if out.is_some() {
            return;
        }
        if let Draw::Glyphs { text, size, glyphs, color, .. } = d {
            if !text.is_empty() {
                out = Some(format!(
                    "Glyphs text={:?} size={size} n_glyphs={} color=({},{},{})",
                    text.chars().take(48).collect::<String>(),
                    glyphs.len(),
                    color.0,
                    color.1,
                    color.2
                ));
            }
        }
    });
    out.unwrap_or_else(|| "sin Glyphs con texto".into())
}

fn walk(draws: &[Draw], f: &mut dyn FnMut(&Draw)) {
    for d in draws {
        f(d);
        match d {
            Draw::Turned { items, .. } | Draw::Rotated { items, .. } | Draw::Clip { items, .. } | Draw::Figure { draws: items, .. } => {
                walk(items, f);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_parrafo_produce_display_list_con_glyphs() {
        let _db = FontDb::global();
        let doc = Document::from_text("Párrafo de prueba.");
        let mut cache = LayoutCache::new();
        let lay = layout(&doc, &mut cache, &LayoutOptions::default());
        assert_eq!(lay.pages.len(), 1);
        let draws = page_display(&doc, &lay.pages[0], &DisplayOptions::default());
        let mut has_glyphs = false;
        walk(&draws, &mut |d| {
            if matches!(d, Draw::Glyphs { text, .. } if !text.is_empty()) {
                has_glyphs = true;
            }
        });
        assert!(has_glyphs, "se esperaba Draw::Glyphs con texto; draws={}", draws.len());
        let s = Session::new(doc);
        assert!(!s.doc.plain_text(StoryRef::Body).is_empty());
    }
}
