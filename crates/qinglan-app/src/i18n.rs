//! The window's texts in Chinese and English; the system's first display language picks one (L1 task 17). Page
//! content is never translated, only what the browser itself says.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Zh,
    En,
}

/// Every text the browser shows. `{}` marks where one value goes (see [`fill`]).
pub struct Strings {
    pub lang_tag: &'static str,
    pub app_name: &'static str,
    pub connecting: &'static str,
    pub loading_sheets: &'static str,
    pub loading_images: &'static str,
    pub stopped: &'static str,
    pub insecure: &'static str,
    pub notice_incomplete: &'static str,
    pub notice_too_large: &'static str,
    pub notice_sheets: &'static str,
    pub mailto_caption: &'static str,
    pub mailto_text: &'static str,
    pub js_blocked: &'static str,
    pub no_font_support: &'static str,
    // Error pages.
    pub err_title: &'static str,
    pub err_internal_title: &'static str,
    pub err_internal_body: &'static str,
    pub err_unsupported_title: &'static str,
    pub err_unsupported_body: &'static str,
    pub err_bad_address_title: &'static str,
    pub err_bad_address_body: &'static str,
    pub err_http_body: &'static str,
    pub err_refused_body: &'static str,
    pub err_timeout: &'static str,
    pub err_dns: &'static str,
    pub err_connect: &'static str,
    pub err_tls: &'static str,
    pub err_too_large: &'static str,
    pub err_redirects: &'static str,
    pub err_protocol: &'static str,
    pub err_other: &'static str,
    pub err_hint: &'static str,
    pub err_details: &'static str,
}

pub static ZH: Strings = Strings {
    lang_tag: "zh",
    app_name: "\u{8f7b}\u{89c8}",
    connecting: "\u{6b63}\u{5728}\u{8fde}\u{63a5}\u{2026}",
    loading_sheets: "\u{6b63}\u{5728}\u{52a0}\u{8f7d}\u{6837}\u{5f0f}\u{8868}\u{ff08}\u{8fd8}\u{5269} {} \u{4e2a}\u{ff09}\u{2026}",
    loading_images: "\u{6b63}\u{5728}\u{52a0}\u{8f7d}\u{56fe}\u{7247}\u{ff08}\u{8fd8}\u{5269} {} \u{4e2a}\u{ff09}\u{2026}",
    stopped: "\u{5df2}\u{505c}\u{6b62}",
    insecure: "\u{4e0d}\u{5b89}\u{5168}",
    notice_incomplete: "\u{9875}\u{9762}\u{592a}\u{590d}\u{6742}\u{ff0c}\u{53ea}\u{663e}\u{793a}\u{4e86}\u{4e00}\u{90e8}\u{5206}\u{3002}",
    notice_too_large: "\u{9875}\u{9762}\u{592a}\u{5927}\u{ff0c}\u{53ea}\u{663e}\u{793a}\u{4e86}\u{524d}\u{9762}\u{4e00}\u{90e8}\u{5206}\u{3002}",
    notice_sheets: "\u{6709} {} \u{4e2a}\u{6837}\u{5f0f}\u{8868}\u{6ca1}\u{6709}\u{52a0}\u{8f7d}\u{ff0c}\u{9875}\u{9762}\u{53ef}\u{80fd}\u{548c}\u{539f}\u{6765}\u{4e0d}\u{4e00}\u{6837}\u{3002}",
    mailto_caption: "\u{6253}\u{5f00}\u{90ae}\u{4ef6}\u{7a0b}\u{5e8f}",
    mailto_text: "\u{8fd9}\u{4e2a}\u{94fe}\u{63a5}\u{8981}\u{7528}\u{90ae}\u{4ef6}\u{7a0b}\u{5e8f}\u{5199}\u{4fe1}\u{ff1a}\n\n{}\n\n\u{8981}\u{7ee7}\u{7eed}\u{5417}\u{ff1f}",
    js_blocked: "\u{8f7b}\u{89c8}\u{4e0d}\u{8fd0}\u{884c}\u{811a}\u{672c}\u{ff0c}\u{8fd9}\u{4e2a}\u{94fe}\u{63a5}\u{6ca1}\u{6709}\u{6253}\u{5f00}\u{3002}",
    no_font_support: "\u{627e}\u{4e0d}\u{5230} DirectWrite\u{ff0c}\u{8f7b}\u{89c8}\u{65e0}\u{6cd5}\u{663e}\u{793a}\u{7f51}\u{9875}\u{3002}",
    err_title: "\u{65e0}\u{6cd5}\u{6253}\u{5f00}\u{6b64}\u{9875}",
    err_internal_title: "\u{8fd9}\u{4e2a}\u{7f51}\u{9875}\u{89e6}\u{53d1}\u{4e86}\u{8f7b}\u{89c8}\u{7684}\u{9519}\u{8bef}",
    err_internal_body: "\u{8f7b}\u{89c8}\u{5728}\u{663e}\u{793a}\u{8fd9}\u{4e2a}\u{7f51}\u{9875}\u{65f6}\u{51fa}\u{4e86}\u{95ee}\u{9898}\u{ff0c}\u{7a97}\u{53e3}\u{6ca1}\u{6709}\u{53d7}\u{5f71}\u{54cd}\u{3002}\u{53ef}\u{4ee5}\u{6362}\u{4e00}\u{4e2a}\u{7f51}\u{9875}\u{ff0c}\u{6216}\u{8005}\u{7a0d}\u{540e}\u{518d}\u{8bd5}\u{3002}",
    err_unsupported_title: "\u{6682}\u{4e0d}\u{652f}\u{6301}\u{6253}\u{5f00}\u{8fd9}\u{79cd}\u{6587}\u{4ef6}",
    err_unsupported_body: "\u{7b2c} 1 \u{5c42}\u{6682}\u{4e0d}\u{652f}\u{6301}\u{4e0b}\u{8f7d}\u{6216}\u{663e}\u{793a}\u{8fd9}\u{79cd}\u{6587}\u{4ef6}\u{ff08}{}\u{ff09}\u{3002}",
    err_bad_address_title: "\u{5730}\u{5740}\u{65e0}\u{6cd5}\u{8bc6}\u{522b}",
    err_bad_address_body: "\u{8fd9}\u{4e0d}\u{662f}\u{4e00}\u{4e2a}\u{5b8c}\u{6574}\u{7684}\u{7f51}\u{5740}\u{3002}",
    err_http_body: "\u{670d}\u{52a1}\u{5668}\u{8fd4}\u{56de}\u{4e86} HTTP {}\u{ff0c}\u{6ca1}\u{6709}\u{5185}\u{5bb9}\u{3002}",
    err_refused_body: "\u{8f7b}\u{89c8}\u{6ca1}\u{6709}\u{6253}\u{5f00}\u{8fd9}\u{4e2a}\u{5730}\u{5740}\u{3002}",
    err_timeout: "\u{670d}\u{52a1}\u{5668}\u{592a}\u{4e45}\u{6ca1}\u{6709}\u{56de}\u{5e94}\u{3002}",
    err_dns: "\u{627e}\u{4e0d}\u{5230}\u{8fd9}\u{4e2a}\u{7f51}\u{7ad9}\u{ff0c}\u{8bf7}\u{68c0}\u{67e5}\u{7f51}\u{5740}\u{62fc}\u{5199}\u{548c}\u{7f51}\u{7edc}\u{8fde}\u{63a5}\u{3002}",
    err_connect: "\u{8fde}\u{63a5}\u{4e0d}\u{4e0a}\u{8fd9}\u{4e2a}\u{7f51}\u{7ad9}\u{3002}",
    err_tls: "\u{5b89}\u{5168}\u{8fde}\u{63a5}\u{5931}\u{8d25}\u{ff1a}\u{8bc1}\u{4e66}\u{6216} TLS \u{51fa}\u{4e86}\u{95ee}\u{9898}\u{3002}\u{8f7b}\u{89c8}\u{4e0d}\u{63d0}\u{4f9b}\u{201c}\u{7ee7}\u{7eed}\u{8bbf}\u{95ee}\u{201d}\u{3002}",
    err_too_large: "\u{8fd9}\u{4e2a}\u{9875}\u{9762}\u{592a}\u{5927}\u{4e86}\u{3002}",
    err_redirects: "\u{91cd}\u{5b9a}\u{5411}\u{6b21}\u{6570}\u{592a}\u{591a}\u{3002}",
    err_protocol: "\u{670d}\u{52a1}\u{5668}\u{7684}\u{56de}\u{7b54}\u{8f7b}\u{89c8}\u{770b}\u{4e0d}\u{61c2}\u{3002}",
    err_other: "\u{52a0}\u{8f7d}\u{5931}\u{8d25}\u{3002}",
    err_hint: "\u{68c0}\u{67e5}\u{7f51}\u{5740}\u{540e}\u{6309} F5 \u{91cd}\u{8bd5}\u{3002}",
    err_details: "\u{8be6}\u{7ec6}\u{4fe1}\u{606f}",
};

pub static EN: Strings = Strings {
    lang_tag: "en",
    app_name: "Qinglan",
    connecting: "Connecting\u{2026}",
    loading_sheets: "Loading style sheets ({} left)\u{2026}",
    loading_images: "Loading images ({} left)\u{2026}",
    stopped: "Stopped",
    insecure: "Not secure",
    notice_incomplete: "This page is too complex; only part of it is shown.",
    notice_too_large: "This page is too large; only the first part is shown.",
    notice_sheets: "{} style sheets did not load; the page may look different.",
    mailto_caption: "Open mail program",
    mailto_text: "This link wants to write a message with your mail program:\n\n{}\n\nContinue?",
    js_blocked: "Qinglan does not run scripts; the link was not opened.",
    no_font_support: "DirectWrite is not available; Qinglan cannot show pages.",
    err_title: "This page can't be opened",
    err_internal_title: "This page triggered an error in Qinglan",
    err_internal_body: "Qinglan ran into a problem while showing this page. The window is not affected. Try another page, or try again later.",
    err_unsupported_title: "This kind of file is not supported",
    err_unsupported_body: "Layer 1 does not download or show this kind of file ({}).",
    err_bad_address_title: "The address is not valid",
    err_bad_address_body: "This is not a complete web address.",
    err_http_body: "The server answered HTTP {} and sent no content.",
    err_refused_body: "Qinglan did not open this address.",
    err_timeout: "The server took too long to answer.",
    err_dns: "The site cannot be found. Check the spelling of the address and your network connection.",
    err_connect: "Cannot connect to the site.",
    err_tls: "The secure connection failed: there is a problem with the certificate or TLS. Qinglan has no \"continue anyway\".",
    err_too_large: "This page is too large.",
    err_redirects: "Too many redirects.",
    err_protocol: "Qinglan cannot understand the server's answer.",
    err_other: "Loading failed.",
    err_hint: "Check the address and press F5 to try again.",
    err_details: "Details",
};

impl Lang {
    /// The language of the system's first display language (Chinese when it starts with "zh", else English).
    pub fn detect() -> Lang {
        if qinglan_win::window::ui_is_chinese() { Lang::Zh } else { Lang::En }
    }

    pub fn strings(self) -> &'static Strings {
        match self {
            Lang::Zh => &ZH,
            Lang::En => &EN,
        }
    }
}

/// Put `value` where the template has `{}`.
pub fn fill(template: &str, value: &str) -> String {
    template.replacen("{}", value, 1)
}

/// The window title for a page: "title - app name" (just the app name when the page has no title).
pub fn window_title(title: &str, s: &Strings) -> String {
    let t = title.trim();
    if t.is_empty() { s.app_name.to_string() } else { format!("{t} - {}", s.app_name) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(s: &Strings) -> Vec<&'static str> {
        vec![
            s.lang_tag,
            s.app_name,
            s.connecting,
            s.loading_sheets,
            s.loading_images,
            s.stopped,
            s.insecure,
            s.notice_incomplete,
            s.notice_too_large,
            s.notice_sheets,
            s.mailto_caption,
            s.mailto_text,
            s.js_blocked,
            s.no_font_support,
            s.err_title,
            s.err_internal_title,
            s.err_internal_body,
            s.err_unsupported_title,
            s.err_unsupported_body,
            s.err_bad_address_title,
            s.err_bad_address_body,
            s.err_http_body,
            s.err_refused_body,
            s.err_timeout,
            s.err_dns,
            s.err_connect,
            s.err_tls,
            s.err_too_large,
            s.err_redirects,
            s.err_protocol,
            s.err_other,
            s.err_hint,
            s.err_details,
        ]
    }

    #[test]
    fn every_text_is_filled_in_both_languages() {
        let (zh, en) = (all(&ZH), all(&EN));
        assert_eq!(zh.len(), en.len());
        for (z, e) in zh.iter().zip(&en) {
            assert!(!z.is_empty() && !e.is_empty());
            assert_ne!(z, e, "{z} was not translated");
            // The same placeholders in both.
            assert_eq!(z.matches("{}").count(), e.matches("{}").count(), "{z} / {e}");
        }
    }

    #[test]
    fn chinese_is_chinese_and_english_is_ascii_prose() {
        assert!(ZH.app_name.chars().all(|c| !c.is_ascii()));
        assert!(all(&ZH).iter().skip(1).all(|t| !t.is_ascii()));
        // The English table uses no CJK; its only non-ASCII characters are the ellipsis.
        assert!(all(&EN).iter().all(|t| t.chars().all(|c| c.is_ascii() || c == '\u{2026}')));
    }

    #[test]
    fn titles_have_the_app_name_after_a_dash() {
        assert_eq!(window_title("  \u{7ef4}\u{57fa}\u{767e}\u{79d1} ", &ZH), "\u{7ef4}\u{57fa}\u{767e}\u{79d1} - \u{8f7b}\u{89c8}");
        assert_eq!(window_title("Example", &EN), "Example - Qinglan");
        assert_eq!(window_title("", &EN), "Qinglan");
        assert_eq!(window_title("   ", &ZH), ZH.app_name);
    }

    #[test]
    fn fill_replaces_the_first_placeholder() {
        assert_eq!(fill("a {} b", "x"), "a x b");
        assert_eq!(fill("none", "x"), "none");
    }
}
