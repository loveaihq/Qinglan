//! Windows platform tests (step 1b-3): legacy decoding, font lookup, shaping, line breaks,
//! metrics, glyph masks, caches. They need the fonts of a normal Windows install.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::rc::Rc;

use qinglan_engine::css::types::{FontFamily, GenericFamily};
use qinglan_engine::platform::{FontId, FontQuery, FontSlant, Glyph, Platform};
use qinglan_win::{WinPlatform, code_page_for, decode_legacy};

fn named(n: &str) -> Vec<FontFamily> {
    vec![FontFamily::Named(Rc::from(n))]
}
fn generic(g: GenericFamily) -> Vec<FontFamily> {
    vec![FontFamily::Generic(g)]
}
fn q<'a>(f: &'a [FontFamily], lang: Option<&'a str>) -> FontQuery<'a> {
    FontQuery { families: f, weight: 400.0, slant: FontSlant::Upright, lang }
}
fn plat() -> WinPlatform {
    WinPlatform::new().expect("DirectWrite")
}
fn family_of(p: &WinPlatform, f: &[FontFamily], lang: Option<&str>) -> String {
    p.family_name(p.resolve_font(&q(f, lang)).unwrap()).unwrap()
}
fn sans() -> Vec<FontFamily> {
    generic(GenericFamily::SansSerif)
}

#[test]
fn decode_gbk_big5_sjis_euckr_eucjp() {
    assert_eq!(decode_legacy("GBK", &[0xD6, 0xD0, 0xCE, 0xC4]).as_deref(), Some("中文"));
    assert_eq!(decode_legacy("gb2312", &[0xD6, 0xD0, 0xCE, 0xC4]).as_deref(), Some("中文"));
    assert_eq!(decode_legacy("gb18030", &[b'a', 0xD6, 0xD0, 0x81, 0x30, 0x81, 0x30]).as_deref(), Some("a中\u{80}"));
    assert_eq!(decode_legacy("Big5", &[0xA4, 0xA4, 0xA4, 0xE5]).as_deref(), Some("中文"));
    assert_eq!(decode_legacy("Shift_JIS", &[0x93, 0xFA, 0x96, 0x7B, 0x8C, 0xEA]).as_deref(), Some("日本語"));
    assert_eq!(decode_legacy("Shift_JIS", &[0xB1, 0xB2]).as_deref(), Some("ｱｲ"));
    assert_eq!(decode_legacy("EUC-KR", &[0xC7, 0xD1, 0xB1, 0xB9, 0xBE, 0xEE]).as_deref(), Some("한국어"));
    assert_eq!(decode_legacy("EUC-JP", &[0xC6, 0xFC, 0xCB, 0xDC, 0xB8, 0xEC]).as_deref(), Some("日本語"));
    assert_eq!(decode_legacy("windows-1251", &[0xCF, 0xF0, 0xE8]).as_deref(), Some("При"));
    assert_eq!(decode_legacy("GBK", &[]).as_deref(), Some(""));
    assert_eq!(decode_legacy("GBK", b"plain ascii").as_deref(), Some("plain ascii"));
}

#[test]
fn decode_bad_bytes_become_fffd() {
    assert_eq!(decode_legacy("GBK", &[0x41, 0xFF, 0x42]).as_deref(), Some("A\u{FFFD}B"));
    assert_eq!(decode_legacy("GBK", &[0x41, 0xD6]).as_deref(), Some("A\u{FFFD}"));
    assert_eq!(decode_legacy("GBK", &[0xD6, 0xD0, 0xFF, 0xD6, 0xD0]).as_deref(), Some("中\u{FFFD}中"));
    assert_eq!(decode_legacy("Big5", &[0xA4, 0xA4, 0xFF, 0xA4, 0xE5]).as_deref(), Some("中\u{FFFD}文"));
    assert_eq!(decode_legacy("Shift_JIS", &[0x41, 0x85, 0x30, 0x42]).as_deref(), Some("A\u{FFFD}0B"));
    assert_eq!(decode_legacy("EUC-KR", &[0xC7, 0xD1, 0xFF, 0xB1, 0xB9]).as_deref(), Some("한\u{FFFD}국"));
}

#[test]
fn decode_table() {
    assert_eq!(code_page_for("GBK"), Some(54936));
    assert_eq!(code_page_for("gb2312"), Some(54936));
    assert_eq!(code_page_for("gb18030"), Some(54936));
    assert_eq!(code_page_for("Big5"), Some(950));
    assert_eq!(code_page_for("Shift_JIS"), Some(932));
    assert_eq!(code_page_for("EUC-KR"), Some(949));
    assert_eq!(code_page_for("KOI8-R"), Some(20866));
    assert_eq!(code_page_for("ISO-8859-16"), None);
    assert!(decode_legacy("ISO-8859-16", b"x").is_none());
    assert!(decode_legacy("UTF-8", b"x").is_none());
}

#[test]
fn generic_families_map_to_windows_fonts() {
    let p = plat();
    assert_eq!(family_of(&p, &generic(GenericFamily::SansSerif), None), "Segoe UI");
    assert_eq!(family_of(&p, &generic(GenericFamily::SystemUi), None), "Segoe UI");
    assert_eq!(family_of(&p, &generic(GenericFamily::Serif), None), "Times New Roman");
    assert_eq!(family_of(&p, &generic(GenericFamily::Monospace), None), "Consolas");
    // Unknown names fall through the list; a list with nothing installed gives the default font.
    let list = vec![FontFamily::Named(Rc::from("No Such Font 42")), FontFamily::Generic(GenericFamily::Monospace)];
    assert_eq!(family_of(&p, &list, None), "Consolas");
    assert_eq!(family_of(&p, &named("No Such Font 42"), None), "Segoe UI");
}

#[test]
fn chinese_family_names_are_found() {
    let p = plat();
    let yahei = p.resolve_font(&q(&named("Microsoft YaHei"), None)).unwrap();
    if p.family_name(yahei).as_deref() == Some("Microsoft YaHei") {
        assert_eq!(p.resolve_font(&q(&named("微软雅黑"), None)), Some(yahei));
    }
    let simsun = p.resolve_font(&q(&named("SimSun"), None)).unwrap();
    if p.family_name(simsun).as_deref() == Some("SimSun") {
        assert_eq!(p.resolve_font(&q(&named("宋体"), None)), Some(simsun));
    }
    println!("微软雅黑 -> {}, 宋体 -> {}", family_of(&p, &named("微软雅黑"), None), family_of(&p, &named("宋体"), None));
}

#[test]
fn shape_latin() {
    let p = plat();
    let f = sans();
    let g = p.shape("Hello", &q(&f, Some("en")), 16.0);
    assert_eq!(g.len(), 5);
    assert!(g.iter().all(|x| x.glyph != 0 && x.advance > 3.0));
    let w: f32 = g.iter().map(|x| x.advance).sum();
    assert!((30.0..48.0).contains(&w), "width {w}");
    for (i, x) in g.iter().enumerate() {
        assert_eq!((x.cluster_start, x.cluster_end), (i as u32, i as u32 + 1));
        assert_eq!(x.font, g[0].font);
    }
    assert!(p.shape("", &q(&f, None), 16.0).is_empty());
}

#[test]
fn shape_cjk_uses_fallback_font() {
    let p = plat();
    let f = sans();
    let g = p.shape("中文", &q(&f, Some("zh-Hans")), 20.0);
    assert_eq!(g.len(), 2);
    assert!(g.iter().all(|x| x.glyph != 0), "{g:?}");
    assert!(g.iter().all(|x| (x.advance - 20.0).abs() < 3.0), "{g:?}");
    let primary = p.resolve_font(&q(&f, Some("zh-Hans"))).unwrap();
    assert_ne!(g[0].font, primary, "Han characters must not come from Segoe UI");
    assert_eq!(g[0].font, g[1].font);
    println!("zh-Hans fallback font: {}", p.family_name(g[0].font).unwrap());
}

#[test]
fn language_picks_cjk_glyphs() {
    let p = plat();
    let f = sans();
    let mut names = Vec::new();
    for lang in ["zh-Hans", "zh-CN", "zh-Hant", "zh-TW", "ja", "ko"] {
        let g = p.shape("直骨海", &q(&f, Some(lang)), 20.0);
        assert_eq!(g.len(), 3, "{lang}");
        assert!(g.iter().all(|x| x.glyph != 0), "{lang}: {g:?}");
        let fam = p.family_name(g[0].font).unwrap();
        println!("{lang:8} -> {fam} glyphs {:?}", g.iter().map(|x| x.glyph).collect::<Vec<_>>());
        names.push((lang, fam));
    }
    let get = |l: &str| names.iter().find(|n| n.0 == l).unwrap().1.clone();
    assert_eq!(get("zh-Hans"), get("zh-CN"));
    assert_eq!(get("zh-Hant"), get("zh-TW"));
    let mut distinct: Vec<&String> = names.iter().map(|n| &n.1).collect();
    distinct.sort();
    distinct.dedup();
    assert!(distinct.len() >= 2, "zh-Hans, zh-Hant, ja and ko should not all pick the same font: {names:?}");
}

#[test]
fn shape_japanese_and_korean() {
    let p = plat();
    let f = sans();
    for (lang, text) in [("ja", "こんにちは、世界"), ("ko", "안녕하세요")] {
        let g = p.shape(text, &q(&f, Some(lang)), 18.0);
        assert!(!g.is_empty());
        assert!(g.iter().all(|x| x.glyph != 0), "{lang}: {g:?}");
        assert_eq!(g.last().unwrap().cluster_end as usize, text.len());
        println!("{lang}: font {}", p.family_name(g[0].font).unwrap());
    }
}

#[test]
fn mixed_text_clusters_cover_the_text() {
    let p = plat();
    let f = sans();
    let text = "Hello 世界, abc 😀 e\u{301}!";
    let g = p.shape(text, &q(&f, Some("zh-Hans")), 16.0);
    assert!(g.first().unwrap().cluster_start == 0 && g.last().unwrap().cluster_end as usize == text.len());
    let mut prev = 0u32;
    for x in &g {
        assert!(x.cluster_start >= prev, "cluster ranges went backwards: {g:?}");
        assert!(text.is_char_boundary(x.cluster_start as usize) && text.is_char_boundary(x.cluster_end as usize));
        prev = x.cluster_start;
    }
    let primary = p.resolve_font(&q(&f, Some("zh-Hans"))).unwrap();
    assert_eq!(g[0].font, primary);
    let han = g.iter().find(|x| text[x.cluster_start as usize..].starts_with('世')).unwrap();
    assert_ne!(han.font, primary);
}

#[test]
fn control_characters_do_not_produce_missing_glyphs() {
    let p = plat();
    let f = sans();
    let g = p.shape("a\tb\nc\u{200B}d", &q(&f, None), 16.0);
    println!("{g:?}");
    assert!(g.iter().all(|x| x.glyph != 0), "{g:?}");
}

#[test]
fn line_breaks_follow_uax14() {
    let p = plat();
    let t = "Hello world 你好，世界。ab";
    let b = p.line_breaks(t, Some("zh-Hans"));
    let off = |needle: &str| t.find(needle).unwrap();
    assert!(b.contains(&6), "after the space: {b:?}");
    assert!(b.contains(&off("好")), "between ideographs: {b:?}");
    assert!(!b.contains(&off("，")), "no break before a comma: {b:?}");
    assert!(!b.contains(&off("。")), "no break before a full stop: {b:?}");
    assert!(b.contains(&off("世")), "after the comma: {b:?}");
    assert!(b.windows(2).all(|w| w[0] < w[1]) && b.iter().all(|&o| o > 0 && o < t.len() && t.is_char_boundary(o)));
    assert!(p.line_breaks("x", None).is_empty());
}

#[test]
fn metrics_are_sane() {
    let p = plat();
    let f = sans();
    let font = p.resolve_font(&q(&f, None)).unwrap();
    let m = p.font_metrics(font, 100.0);
    println!("{m:?}");
    assert!((80.0..120.0).contains(&m.ascent) && (15.0..40.0).contains(&m.descent), "{m:?}");
    assert!(m.line_gap >= 0.0 && m.line_gap < 20.0);
    let x = m.x_height.expect("x-height");
    assert!((40.0..60.0).contains(&x), "x-height {x}");
    let z = m.zero_advance.expect("zero advance");
    assert!((45.0..65.0).contains(&z), "zero advance {z}");
    assert!(m.underline_thickness > 0.0 && m.underline_offset > 0.0 && m.underline_offset < 40.0, "{m:?}");
    assert!(m.strikeout_thickness > 0.0 && m.strikeout_offset > 10.0 && m.strikeout_offset < 50.0, "{m:?}");
    // The advance of "0" agrees with shaping.
    let adv = p.shape("0", &q(&f, None), 100.0)[0].advance;
    assert!((adv - z).abs() < 1.0, "{adv} vs {z}");
    // Linear in the size.
    let m2 = p.font_metrics(font, 50.0);
    assert!((m2.ascent * 2.0 - m.ascent).abs() < 0.01);
    assert_eq!(p.font_metrics(FontId(9999), 10.0), Default::default());
}

#[test]
fn glyph_masks() {
    let p = plat();
    let f = sans();
    let g = p.shape("A ", &q(&f, None), 32.0);
    let a = p.glyph_mask(g[0].font, g[0].glyph, 32.0, 0).expect("A has a mask");
    assert!(a.width > 10 && a.width < 40 && a.height > 15 && a.height < 40, "{}x{}", a.width, a.height);
    assert_eq!(a.alpha.len(), (a.width * a.height) as usize);
    assert!(a.alpha.iter().any(|&v| v > 200) && a.alpha.contains(&0));
    assert!(a.top < 0 && a.top > -40 && a.left >= -2, "left {} top {}", a.left, a.top);
    assert!(p.glyph_mask(g[1].font, g[1].glyph, 32.0, 0).is_none(), "space draws nothing");
    // Quarter-pixel positions give different coverage.
    let b = p.glyph_mask(g[0].font, g[0].glyph, 32.0, 2).unwrap();
    assert!(a.alpha != b.alpha || a.left != b.left);
    // A cache hit returns the same pixels.
    assert_eq!(p.glyph_mask(g[0].font, g[0].glyph, 32.0, 0).unwrap(), a);
    assert!(p.cache_stats().glyph_hits >= 1);
    assert!(p.glyph_mask(g[0].font, 0x1_0000, 32.0, 0).is_none());
}

#[test]
fn bold_and_italic_use_simulation_when_the_font_lacks_them() {
    let p = plat();
    let fam = named("SimSun");
    let reg = p.resolve_font(&q(&fam, None)).unwrap();
    if p.family_name(reg).as_deref() != Some("SimSun") {
        return;
    }
    let bold_q = FontQuery { families: &fam, weight: 700.0, slant: FontSlant::Upright, lang: Some("zh-Hans") };
    let bold = p.resolve_font(&bold_q).unwrap();
    let ital = p.resolve_font(&FontQuery { families: &fam, weight: 400.0, slant: FontSlant::Italic, lang: None }).unwrap();
    assert_eq!(p.simulations(reg), 0);
    assert_eq!(p.simulations(bold) & 1, 1, "synthetic bold");
    assert_eq!(p.simulations(ital) & 2, 2, "synthetic oblique");
    let gr = p.shape("永", &q(&fam, Some("zh-Hans")), 32.0)[0];
    let gb = p.shape("永", &bold_q, 32.0)[0];
    let mr = p.glyph_mask(gr.font, gr.glyph, 32.0, 0).unwrap();
    let mb = p.glyph_mask(gb.font, gb.glyph, 32.0, 0).unwrap();
    let ink = |m: &qinglan_engine::platform::GlyphMask| m.alpha.iter().map(|&v| u64::from(v)).sum::<u64>();
    assert!(ink(&mb) > ink(&mr), "bold has more ink: {} vs {}", ink(&mb), ink(&mr));
    // A font that has real bold faces does not simulate.
    let s = sans();
    let sans_bold = p.resolve_font(&FontQuery { families: &s, weight: 700.0, slant: FontSlant::Upright, lang: None }).unwrap();
    assert_eq!(p.simulations(sans_bold), 0);
    assert_ne!(sans_bold, p.resolve_font(&q(&s, None)).unwrap());
}

#[test]
fn shape_cache_hits_and_fonts_are_interned() {
    let p = plat();
    let f = sans();
    let first = p.shape("缓存测试 cache test", &q(&f, Some("zh-Hans")), 16.0);
    let s1 = p.cache_stats();
    let second = p.shape("缓存测试 cache test", &q(&f, Some("zh-Hans")), 16.0);
    let s2 = p.cache_stats();
    assert_eq!(first, second);
    assert_eq!(s2.shape_hits, s1.shape_hits + 1);
    assert_eq!(s2.shape_misses, s1.shape_misses);
    assert!(s2.shape_bytes > 0 && s2.shape_entries >= 1);
    // A different size is a different entry.
    p.shape("缓存测试 cache test", &q(&f, Some("zh-Hans")), 17.0);
    assert_eq!(p.cache_stats().shape_misses, s2.shape_misses + 1);
    // Shaping again from scratch gives the same FontIds (the face is found again, not duplicated).
    p.clear_shape_cache();
    let fonts_before = p.cache_stats().fonts;
    let third = p.shape("缓存测试 cache test", &q(&f, Some("zh-Hans")), 16.0);
    assert_eq!(first, third);
    assert_eq!(p.cache_stats().fonts, fonts_before);
}

#[test]
fn caches_stay_inside_their_budgets() {
    let p = plat();
    let f = sans();
    // Many distinct strings: the shaping cache must not grow past its budget.
    for i in 0..4000 {
        let s = format!("line {i} {}", "word ".repeat(40));
        p.shape(&s, &q(&f, None), 14.0);
    }
    let s = p.cache_stats();
    assert!(s.shape_bytes <= 8 << 20, "{s:?}");
    // Many distinct glyph masks.
    let g = p.shape("A", &q(&f, None), 16.0)[0];
    for size in 8..300 {
        for sub in 0..4u8 {
            p.glyph_mask(g.font, g.glyph, size as f32 * 1.5, sub);
        }
    }
    let s = p.cache_stats();
    assert!(s.glyph_bytes <= 8 << 20, "{s:?}");
    assert!(s.glyph_entries > 100);
}

#[test]
fn long_text_is_shaped_in_chunks() {
    let p = plat();
    let f = sans();
    let text = "中文长文本，".repeat(5000);
    let g: Vec<Glyph> = p.shape(&text, &q(&f, Some("zh-Hans")), 16.0);
    assert_eq!(g.len(), 6 * 5000);
    assert!(g.iter().all(|x| x.glyph != 0));
    assert_eq!(g.last().unwrap().cluster_end as usize, text.len());
}

#[test]
fn all_kinds_of_text_keep_the_cluster_invariants() {
    let p = plat();
    let f = sans();
    let texts = [
        "مرحبا بالعالم",
        "שלום עולם",
        "สวัสดีชาวโลก ภาษาไทย",
        "नमस्ते दुनिया",
        "Привет, мир! Ελληνικά",
        "😀👨‍👩‍👧‍👦🇨🇳 flags and ZWJ sequences",
        "\u{301}leading mark, \u{200D}ZWJ, \u{FE0F}VS, \u{0}NUL, \u{1}SOH",
        "漢\u{E0100}字 with IVS, 𠀀 extension B, ｱｲｳ halfwidth, ａｂｃ fullwidth",
        "a\u{2028}b\u{2029}c\u{85}d",
        "   \t\t  \n\n ",
        "한글과 日本語と中文、English mixed.",
    ];
    for lang in [None, Some("ar"), Some("zh-Hans"), Some("x-bad lang!"), Some("")] {
        for t in texts {
            let g = p.shape(t, &q(&f, lang), 15.0);
            assert!(!g.is_empty(), "{t:?}");
            assert_eq!(g.first().unwrap().cluster_start, 0, "{t:?}");
            assert_eq!(g.last().unwrap().cluster_end as usize, t.len(), "{t:?}");
            let mut prev = 0;
            for x in &g {
                assert!(x.cluster_start >= prev && x.cluster_end >= x.cluster_start, "{t:?}: {g:?}");
                assert!(t.is_char_boundary(x.cluster_start as usize) && t.is_char_boundary(x.cluster_end as usize), "{t:?}");
                assert!(x.advance.is_finite() && x.advance >= 0.0 && x.offset_x.is_finite() && x.offset_y.is_finite());
                prev = x.cluster_start;
            }
            let b = p.line_breaks(t, lang);
            assert!(b.windows(2).all(|w| w[0] < w[1]) && b.iter().all(|&o| o > 0 && o < t.len() && t.is_char_boundary(o)), "{t:?}: {b:?}");
        }
    }
    // Absurd sizes and weights do not crash.
    for size in [0.0, -3.0, f32::NAN, f32::INFINITY, 1e9] {
        let _ = p.shape("abc 中文", &q(&f, None), size);
        let _ = p.font_metrics(p.resolve_font(&q(&f, None)).unwrap(), size);
    }
    let odd = FontQuery { families: &f, weight: f32::NAN, slant: FontSlant::Oblique, lang: None };
    assert!(p.resolve_font(&odd).is_some());
    assert!(p.resolve_font(&FontQuery { families: &[], weight: 1e9, slant: FontSlant::Italic, lang: None }).is_some());
}

/// A long run is cut into pieces for DirectWrite; the cut must not fall between a character and its
/// combining mark (the two would end up in different clusters).
#[test]
fn long_runs_are_not_cut_inside_a_cluster() {
    let p = plat();
    let f = sans();
    // 8191 letters, then "e" + U+0301 exactly where the first piece would end (8192 units).
    for filler in ["a", "ab "] {
        let n = 8191 / filler.len();
        let mut text = filler.repeat(n);
        text.push_str(&"a".repeat(8191 - text.len()));
        text.push_str("e\u{0301}");
        text.push_str(&"b".repeat(100));
        let at = text.find('e').unwrap();
        assert_eq!(text[..at].encode_utf16().count(), 8191);
        let g: Vec<Glyph> = p.shape(&text, &q(&f, Some("en")), 16.0);
        let e_mark = at as u32..(at + 3) as u32;
        assert!(g.iter().any(|x| x.cluster_start == e_mark.start && x.cluster_end == e_mark.end), "{filler:?}: e and U+0301 must share a cluster");
        assert!(g.iter().all(|x| !(x.cluster_start == e_mark.end - 2 && x.cluster_end == e_mark.end)), "{filler:?}: the mark has a cluster of its own");
    }
}

/// Shaping the context-free characters in bulk (`prefetch`) gives the same glyph, advance and cluster
/// as shaping each one alone, for the CJK ideographs, kana, Hangul, CJK and full-width punctuation, in
/// several fonts and languages. (The combining marks of those blocks are not context-free and are left out;
/// the same characters in the same order go through both platforms, because the fallback font a
/// character gets can depend on which fallback fonts were chosen before it.)
#[test]
fn bulk_shaping_matches_single() {
    let ranges: &[(u32, u32)] = &[(0x3000, 0x303F), (0x3040, 0x30FF), (0x3400, 0x4DBF), (0x4E00, 0x9FFF), (0xAC00, 0xD7A3), (0xF900, 0xFAFF), (0xFF01, 0xFF60), (0x20000, 0x2A6DF)];
    let configs: [(Vec<FontFamily>, &str, usize); 5] = [
        (sans(), "zh-Hans", 1),
        (sans(), "ja", 5),
        (sans(), "ko", 5),
        (generic(GenericFamily::Serif), "zh-Hant", 5),
        (named("Microsoft YaHei"), "zh-Hans", 5),
    ];
    for (fam, lang, step) in &configs {
        let chars: Vec<char> = ranges.iter().flat_map(|&(a, b)| (a..=b).step_by(*step)).filter(|c| !matches!(c, 0x302A..=0x302F | 0x3099..=0x309A)).filter_map(char::from_u32).collect();
        let all: String = chars.iter().collect();
        let (single, bulk) = (plat(), plat());
        bulk.prefetch(&all, &[], &q(fam, Some(lang)));
        let stats = bulk.cache_stats();
        assert!(stats.batch_chars as usize > chars.len() / 2, "{lang}: only {} of {} characters were shaped in bulk", stats.batch_chars, chars.len());
        for c in chars {
            let s = c.to_string();
            let a = single.shape(&s, &q(fam, Some(lang)), 16.0);
            let b = bulk.shape(&s, &q(fam, Some(lang)), 16.0);
            // FontIds are numbered in the order the platform meets the fonts, so compare the families.
            assert_eq!(a.len(), b.len(), "{lang}: U+{:04X}", c as u32);
            for (x, y) in a.iter().zip(&b) {
                let same = (x.glyph, x.advance.to_bits(), x.offset_x.to_bits(), x.offset_y.to_bits(), x.cluster_start, x.cluster_end) == (y.glyph, y.advance.to_bits(), y.offset_x.to_bits(), y.offset_y.to_bits(), y.cluster_start, y.cluster_end);
                assert!(same && single.family_name(x.font) == bulk.family_name(y.font), "{lang}: U+{:04X}: {x:?} vs {y:?}", c as u32);
            }
        }
        assert_eq!(single.cache_stats().batch_calls, 0);
    }
}

/// Words are shaped once and reused, also at another size (advances scale linearly).
#[test]
fn words_are_cached_and_scale_with_the_size() {
    let p = plat();
    let f = sans();
    let a = p.shape("the quick brown fox the", &q(&f, Some("en")), 16.0);
    let s1 = p.cache_stats();
    assert!(s1.word_hits >= 2, "the second \"the\" and the repeated spaces are cache hits: {s1:?}");
    let b = p.shape("the quick brown fox the", &q(&f, Some("en")), 24.0);
    let s2 = p.cache_stats();
    assert_eq!(s2.word_misses, s1.word_misses, "no new words at another size");
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!((x.glyph, x.cluster_start, x.cluster_end), (y.glyph, y.cluster_start, y.cluster_end));
        assert!((x.advance * 1.5 - y.advance).abs() < 0.01, "{} {}", x.advance, y.advance);
    }
    // Spaces are glyphs of their own and the clusters cover the text.
    assert_eq!(a.last().unwrap().cluster_end as usize, "the quick brown fox the".len());
}

/// ASCII words shaped in bulk (`prefetch`) are exactly the words shaped one at a time (same glyphs, the
/// same advances to the bit), in several fonts, including words with kerning pairs such as "AV" and "To".
#[test]
fn bulk_word_shaping_matches_single() {
    let mut words: Vec<String> = ["AV", "To", "Yo", "WAVE", "Typography", "fi", "ffl", "office", "1990", "(2011)", "http://example.org/a_b?c=d&e=f", "T.", "Y-", "LTA", "r.", "v,", "AT&T", "{}", "~"]
        .iter()
        .map(|w| w.to_string())
        .collect();
    // A few thousand pseudo-random words over the printable ASCII range.
    let mut x = 12345u32;
    for _ in 0..4000 {
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        let len = 1 + (x >> 28) as usize;
        let mut w = String::new();
        for _ in 0..len {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            w.push(char::from(0x21 + ((x >> 24) % 94) as u8));
        }
        words.push(w);
    }
    words.sort();
    words.dedup();
    let refs: Vec<&str> = words.iter().map(String::as_str).collect();
    for (fam, lang) in [(sans(), "en"), (generic(GenericFamily::Serif), "en"), (generic(GenericFamily::Monospace), "en"), (named("Arial"), "en"), (named("Microsoft YaHei"), "zh-Hans")] {
        let (single, bulk) = (plat(), plat());
        bulk.prefetch("", &refs, &q(&fam, Some(lang)));
        let stats = bulk.cache_stats();
        assert!(stats.word_prefetched as usize > refs.len() / 2, "{lang}: only {} of {} words were taken from the bulk call", stats.word_prefetched, refs.len());
        for w in &refs {
            let a = single.shape(w, &q(&fam, Some(lang)), 16.0);
            let b = bulk.shape(w, &q(&fam, Some(lang)), 16.0);
            assert_eq!(a.len(), b.len(), "{w:?}");
            for (x, y) in a.iter().zip(&b) {
                assert!((x.glyph, x.advance.to_bits(), x.offset_x.to_bits(), x.cluster_start, x.cluster_end) == (y.glyph, y.advance.to_bits(), y.offset_x.to_bits(), y.cluster_start, y.cluster_end), "{w:?}: {x:?} vs {y:?}");
            }
        }
        assert_eq!(bulk.cache_stats().word_misses, 0, "every word came from the bulk call");
    }
}

/// The prewarm opens the common fonts on a background thread and leaves a working platform behind it.
#[test]
fn prewarm_opens_fonts_on_another_thread() {
    let n = std::thread::spawn(qinglan_win::prewarm).join().unwrap();
    assert!(n >= 6, "only {n} font faces opened (Segoe UI, Consolas and Times New Roman, regular and bold, are always there)");
    // A second call has nothing left to do.
    assert_eq!(qinglan_win::prewarm(), 0);
    // A platform made afterwards works as before.
    let p = plat();
    let g = p.shape("Hello \u{4e2d}\u{6587}", &q(&sans(), Some("zh-Hans")), 16.0);
    assert!(g.iter().all(|g| g.glyph != 0));
}
