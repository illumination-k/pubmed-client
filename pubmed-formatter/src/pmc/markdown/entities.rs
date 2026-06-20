use std::sync::OnceLock;

static HTML_ENTITIES: &[(&str, &str)] = &[
    // Basic HTML entities
    ("&amp;", "&"),
    ("&lt;", "<"),
    ("&gt;", ">"),
    ("&quot;", "\""),
    ("&#x27;", "'"),
    ("&apos;", "'"),
    // Quotation marks
    ("&#8217;", "'"),  // right single quotation mark
    ("&#8216;", "'"),  // left single quotation mark
    ("&#8220;", "\""), // left double quotation mark
    ("&#8221;", "\""), // right double quotation mark
    ("&rsquo;", "'"),  // right single quote
    ("&lsquo;", "'"),  // left single quote
    ("&rdquo;", "\""), // right double quote
    ("&ldquo;", "\""), // left double quote
    // Dashes and spacing
    ("&#8211;", "-"),  // en dash
    ("&#8212;", "--"), // em dash
    ("&#160;", " "),   // non-breaking space
    ("&nbsp;", " "),   // non-breaking space
    ("&ndash;", "-"),  // en dash
    ("&mdash;", "--"), // em dash
    // Special punctuation
    ("&#8230;", "..."),  // ellipsis
    ("&hellip;", "..."), // ellipsis
    // Symbols
    ("&#8482;", "(TM)"), // trademark
    ("&#174;", "(R)"),   // registered trademark
    ("&#169;", "(C)"),   // copyright
    ("&trade;", "(TM)"), // trademark
    ("&reg;", "(R)"),    // registered trademark
    ("&copy;", "(C)"),   // copyright
    // Currency (simplified)
    ("&#8364;", "EUR"), // euro
    ("&#163;", "GBP"),  // pound
    ("&#165;", "JPY"),  // yen
    // Mathematical symbols
    ("&#8722;", "-"),  // minus sign
    ("&#215;", "x"),   // multiplication sign
    ("&#247;", "/"),   // division sign
    ("&#177;", "±"),   // plus-minus sign
    ("&times;", "x"),  // multiplication sign
    ("&divide;", "/"), // division sign
    ("&plusmn;", "±"), // plus-minus sign
    // Greek letters (common in scientific texts)
    ("&#945;", "α"),    // alpha
    ("&#946;", "β"),    // beta
    ("&#947;", "γ"),    // gamma
    ("&#948;", "δ"),    // delta
    ("&#949;", "ε"),    // epsilon
    ("&#956;", "μ"),    // mu
    ("&#960;", "π"),    // pi
    ("&#963;", "σ"),    // sigma
    ("&alpha;", "α"),   // alpha
    ("&beta;", "β"),    // beta
    ("&gamma;", "γ"),   // gamma
    ("&delta;", "δ"),   // delta
    ("&epsilon;", "ε"), // epsilon
    ("&mu;", "μ"),      // mu
    ("&pi;", "π"),      // pi
    ("&sigma;", "σ"),   // sigma
];

pub(super) fn clean_content(content: &str) -> String {
    static TAG_RE: OnceLock<Option<regex::Regex>> = OnceLock::new();
    let mut cleaned = match TAG_RE.get_or_init(|| regex::Regex::new(r"<[^>]*>").ok()) {
        Some(re) => re.replace_all(content, "").into_owned(),
        None => content.to_string(),
    };

    for (entity, replacement) in HTML_ENTITIES {
        cleaned = cleaned.replace(entity, replacement);
    }

    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}
