use annotate_snippets::{
    display_list::{DisplayList, FormatOptions},
    snippet::{Annotation, AnnotationType, Slice, Snippet, SourceAnnotation},
};
use serde::{Deserialize};

#[derive(Deserialize, Debug)]
struct Replacement {
    value: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
enum RuleType {
    Misspelling,
    Typographical,
    Style,
    Grammar,
    Uncategorized,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Rule {
    issue_type: RuleType,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Issue {
    message: String,
    short_message: String,
    replacements: Vec<Replacement>,
    offset: usize,
    length: usize,
    rule: Rule,
}

#[derive(Deserialize, Debug)]
struct ApiResult {
    matches: Vec<Issue>,
}

fn normalize(s: &mut str) -> &str {
    s.make_ascii_lowercase();
    s.strip_suffix('.').unwrap_or(s)
}

fn main() {
    let text = std::env::args().skip(1).next().expect("pass an argument");
    let client = reqwest::blocking::Client::new();
    let body = serde_urlencoded::to_string(&[
        ("text", &text[..]),
        ("language", "en-GB"),
    ]).unwrap();
    let res = client.post("https://api.languagetool.org/v2/check").body(body).send().expect("web request failed");

    for mut issue in res.json::<ApiResult>().expect("JSON parsing failure").matches {
        let annotation_type = match issue.rule.issue_type {
            RuleType::Misspelling | RuleType::Grammar => AnnotationType::Error,
            _ => AnnotationType::Warning,
        };
        let (short_message, message) = if issue.short_message.is_empty() {
            (normalize(&mut issue.message), "here")
        } else {
            (normalize(&mut issue.short_message), normalize(&mut issue.message))
        };
        let longer_lived_value;
        let snippet = Snippet {
            title: Some(Annotation {
                label: Some(short_message),
                id: None,
                annotation_type,
            }),
            footer: if issue.replacements.is_empty() {
                Vec::new()
            } else {
                longer_lived_value = format!("try replacing the text with \"{}\"", issue.replacements[0].value);
                vec![Annotation {
                    id: None,
                    label: Some(&longer_lived_value),
                    annotation_type: AnnotationType::Help,
                }]
            },
            slices: vec![Slice {
                source: &text,
                line_start: 1,
                origin: None,
                fold: false,
                annotations: vec![
                    SourceAnnotation {
                        label: message,
                        annotation_type,
                        range: (issue.offset, issue.offset+issue.length),
                    },
                ],
            }],
            opt: FormatOptions {
                color: true,
                ..Default::default()
            },
        };
        println!("{}", DisplayList::from(snippet));
    }
}
