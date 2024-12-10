// SPDX-FileCopyrightText: 2021 Uwe Jugel
//
// SPDX-License-Identifier: MIT

use std::sync::LazyLock;

use regex::Regex;

const H1: &str = "H1";
const H2: &str = "H2";
const H3: &str = "H3";
const UL: &str = "BULLET";
const OL: &str = "LIST";
const CODE: &str = "CODE";

static SECTIONS: LazyLock<Vec<Section>> = LazyLock::new(|| {
    vec![
        Section {
            name: H1,
            re: Regex::new(r"^(#\s+)(.*)(\s*)$").unwrap(),
            sub: "<big><big><big>$2</big></big></big>",
        },
        Section {
            name: H2,
            re: Regex::new(r"^(##\s+)(.*)(\s*)$").unwrap(),
            sub: "<big><big>$2</big></big>",
        },
        Section {
            name: H3,
            re: Regex::new(r"^(###\s+)(.*)(\s*)$").unwrap(),
            sub: "<big>$2</big>",
        },
        Section {
            name: UL,
            re: Regex::new(r"^(\s*[\*\-]\s)(.*)(\s*)$").unwrap(),
            sub: " • $2",
        },
        Section {
            name: OL,
            re: Regex::new(r"^(\s*[0-9]+\.\s)(.*)(\s*)$").unwrap(),
            sub: " $1$2",
        },
        Section {
            name: CODE,
            re: Regex::new(r"^```[a-z_]*$").unwrap(),
            sub: "<tt>",
        },
    ]
});

static STYLES: LazyLock<Vec<Style>> = LazyLock::new(|| {
    vec![
        Style {
            re: Regex::new(r"(^|[^\*])(\*\*)(.*)(\*\*)").unwrap(),
            sub: "$1<b>$3</b>",
        },
        Style {
            re: Regex::new(r"(^|[^\*])(\*)(.*)(\*)").unwrap(),
            sub: "$1<i>$3</i>",
        },
        Style {
            re: Regex::new(r"(`)([^`]*)(`)").unwrap(),
            sub: "<tt>$2</tt>",
        },
        Style {
            re: Regex::new(r"(!)?(\[)(.*)(\]\()(.+)(\))").unwrap(),
            sub: "<a href='$5'>$3</a>",
        },
    ]
});

static RE_COMMENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*<!--.*-->\s*$").unwrap());

#[derive(Clone)]
struct Section {
    name: &'static str,
    re: Regex,
    sub: &'static str,
}

struct Style {
    re: Regex,
    sub: &'static str,
}

fn escape_line(line: &str, escapes: &[(Regex, &str)]) -> String {
    escapes.iter().fold(line.to_string(), |acc, (re, sub)| {
        re.replace_all(&acc, *sub).to_string()
    })
}

pub fn convert(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();

    let mut output: Vec<String> = Vec::new();
    let mut is_code = false;

    for line in lines {
        if RE_COMMENT.is_match(line) {
            continue;
        }

        let mut result = line.to_string();
        if !is_code {
            result = escape_line(&result, &[]);
        }

        for section in SECTIONS.iter() {
            if section.re.is_match(line) {
                if section.name == "CODE" {
                    is_code = !is_code;
                    result = if is_code {
                        "<tt>".to_string()
                    } else {
                        "</tt>".to_string()
                    };
                } else {
                    result = section.re.replace(line, section.sub).to_string();
                }
                break;
            }
        }

        if !is_code {
            for style in STYLES.iter() {
                result = style.re.replace_all(&result, style.sub).to_string();
            }
        }

        output.push(result);
    }

    output.join("\n")
}
