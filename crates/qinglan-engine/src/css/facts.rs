//! What a document contains, so that rules which cannot match anything in it are dropped before their
//! declarations are parsed (real sites ship tens of thousands of rules, a page uses a few hundred).
//!
//! The test is one-sided: a selector is dropped only when one of its compounds needs a tag, id, class or
//! attribute value that no element of the document has. Whatever might match is kept, so the styles are the
//! same with and without it. Everything is compared as hashes of ASCII-lower-cased text (a collision only
//! keeps a rule that could have been dropped).

use std::collections::HashSet;

use super::selector::{AttrOp, CaseSens, Selector, Simple};
use crate::html::atoms::{FxBuild, Interner, a};
use crate::html::dom::Document;

type Set = HashSet<u64, FxBuild>;

#[derive(Default)]
pub struct DocFacts {
    tags: Set,
    ids: Set,
    classes: Set,
    attr_names: Set,
    /// (attribute name, value) pairs, the value as written.
    attr_pairs: Set,
}

/// FNV-1a over the bytes, upper case letters folded to lower case when `fold`.
fn hash(text: &str, fold: bool) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &b in text.as_bytes() {
        let b = if fold { b.to_ascii_lowercase() } else { b };
        h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn pair(name: &str, value: &str) -> u64 {
    hash(name, true).rotate_left(17) ^ hash(value, false).wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

impl DocFacts {
    pub fn collect(doc: &Document) -> DocFacts {
        let mut f = DocFacts::default();
        for n in 0..doc.nodes.len() {
            let Some(e) = u32::try_from(n).ok().and_then(|n| doc.element(n)) else { continue };
            f.tags.insert(hash(doc.atoms.get(e.name), true));
            for at in &e.attrs {
                let name = doc.atoms.get(at.name);
                f.attr_names.insert(hash(name, true));
                f.attr_pairs.insert(pair(name, &at.value));
                if at.name == a::ID {
                    f.ids.insert(hash(&at.value, false));
                } else if at.name == a::CLASS {
                    for tok in at.value.split([' ', '\t', '\n', '\x0c', '\r']).filter(|t| !t.is_empty()) {
                        f.classes.insert(hash(tok, false));
                    }
                }
            }
        }
        f
    }

    /// Can `sel` match some element of the document? `false` only when it certainly cannot. Ids and classes are
    /// compared case-insensitively in quirks mode: not judged there.
    pub fn possible(&self, sel: &Selector, atoms: &Interner, quirks: bool) -> bool {
        sel.compounds.iter().flat_map(|c| &c.simples).all(|s| match s {
            Simple::Type { lower, .. } => self.tags.contains(&hash(atoms.get(*lower), true)),
            Simple::Id(id) => quirks || self.ids.contains(&hash(atoms.get(*id), false)),
            Simple::Class(c) => quirks || self.classes.contains(&hash(atoms.get(*c), false)),
            Simple::Attr(at) => {
                let name = atoms.get(at.lower);
                let exact = at.case == CaseSens::Sensitive || (at.case == CaseSens::Default && !at.ci_html);
                self.attr_names.contains(&hash(name, true)) && (at.op != AttrOp::Equals || !exact || self.attr_pairs.contains(&pair(name, &at.value)))
            }
            Simple::NeverMatches => false,
            Simple::Universal | Simple::Pseudo(_) => true,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::css::media::MediaEnv;
    use crate::css::stylesheet::{Rule, SheetCtx, Stylesheet, parse_stylesheet};
    use crate::css::syntax::{parse_cvs, parse_cvs_many, serialize_cvs};
    use crate::html::treebuilder::parse_document;

    const SHEET: &str = ".a{color:red} .zz{color:blue} [data-mode=dark]{color:blue} [data-mode=auto]{color:green} #x{color:red} #nope{color:red} \
        .zz,.b{color:red} :is(.zz){color:red} section{color:red} div .zz{color:red} div .a{color:red} [lang]{color:red} [data-mode^=da]{color:red} \
        @media (min-width: 5000px){.a{color:red}} @media (min-width: 100px){.a{color:red}}";

    fn parse(doc: &mut Document, facts: Option<&DocFacts>, env: Option<&MediaEnv>) -> Stylesheet {
        let (mut v, mut b) = (0, 0);
        let mut ctx = SheetCtx { atoms: &mut doc.atoms, quirks: false, var_id: &mut v, block_id: &mut b, facts, env };
        parse_stylesheet(SHEET, &mut ctx)
    }

    fn count(rules: &[Rule]) -> (usize, usize) {
        let (mut style, mut media) = (0, 0);
        for r in rules {
            match r {
                Rule::Style(_) => style += 1,
                Rule::Media(_, inner) => {
                    media += 1;
                    style += count(inner).0;
                }
                _ => {}
            }
        }
        (style, media)
    }

    #[test]
    fn rules_that_need_something_the_document_lacks_are_dropped_before_their_declarations_are_parsed() {
        let mut doc = parse_document("<html data-mode=auto><body class='a b'><div id=x>t</div>".to_string());
        let all = count(&parse(&mut doc, None, None).rules);
        assert_eq!(all, (15, 2));
        let facts = DocFacts::collect(&doc);
        let env = MediaEnv::screen(1280.0, 800.0);
        let kept = parse(&mut doc, Some(&facts), Some(&env));
        // Kept: .a, [data-mode=auto], #x, ".zz,.b" (.b exists), :is(.zz) (not judged), section? no (no <section>),
        // div .a, [lang]? no, [data-mode^=da] (not judged), and the .a inside the applying @media.
        // Dropped: .zz, [data-mode=dark], #nope, section, div .zz, [lang], and the whole @media (min-width: 5000px) block.
        assert_eq!(count(&kept.rules), (8, 1));
        // The facts alone (no environment) keep both @media blocks, but still drop the rules inside that cannot match.
        let facts_only = parse(&mut doc, Some(&facts), None);
        assert_eq!(count(&facts_only.rules), (9, 2));
    }

    #[test]
    fn quirks_mode_does_not_judge_ids_and_classes() {
        let mut doc = parse_document("<body class='A'>".to_string());
        let facts = DocFacts::collect(&doc);
        let (mut v, mut b) = (0, 0);
        let mut ctx = SheetCtx { atoms: &mut doc.atoms, quirks: true, var_id: &mut v, block_id: &mut b, facts: Some(&facts), env: None };
        let s = parse_stylesheet(".a{color:red} .zz{color:red}", &mut ctx);
        assert_eq!(count(&s.rules).0, 2, ".a matches class A in quirks mode");
    }

    #[test]
    fn tokenizing_several_sheets_on_threads_gives_the_same_values_in_the_same_order() {
        let texts: Vec<String> = (0..9).map(|i| format!("a{i}{{color:red;margin:{i}px}}\n").repeat(3000 + i * 500)).collect();
        let slices: Vec<&str> = texts.iter().map(String::as_str).collect();
        assert!(slices.iter().map(|t| t.len()).sum::<usize>() > 256 * 1024, "big enough to use threads");
        let many = parse_cvs_many(&slices);
        assert_eq!(many.len(), 9);
        for (m, t) in many.iter().zip(&slices) {
            assert_eq!(serialize_cvs(m), serialize_cvs(&parse_cvs(t)));
        }
    }
}
