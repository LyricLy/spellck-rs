use annotate_snippets::{
    display_list::{DisplayList, FormatOptions},
    snippet::{Annotation, AnnotationType, Slice, Snippet, SourceAnnotation},
};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Replacement {
    value: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
enum RuleType {
    Misspelling,
    Typographical,
    Style,
    Grammar,
    Inconsistency,
    Uncategorized,
    NonConformance,
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

fn normalize(s: &str) -> String {
    let mut c = s.strip_suffix('.').unwrap_or(s).chars();
    c.next()
        .map(|f| f.to_lowercase().collect::<String>() + c.as_str())
        .unwrap_or_else(String::new)
}

fn main() {
    let text = std::env::args().nth(1).expect("pass an argument");

    let body = serde_urlencoded::to_string(&[("text", &text[..]), ("language", "en-GB")]).unwrap();
    let res = ureq::post("https://api.languagetool.org/v2/check")
        .send_string(&body)
        .expect("web request failed");

    for issue in res
        .into_json::<ApiResult>()
        .expect("JSON parsing failure")
        .matches
    {
        let annotation_type = match issue.rule.issue_type {
            RuleType::Misspelling | RuleType::Grammar => AnnotationType::Error,
            _ => AnnotationType::Warning,
        };
        let (short_message, message) = if issue.short_message.is_empty() {
            (normalize(&issue.message), "here".to_owned())
        } else {
            (normalize(&issue.short_message), normalize(&issue.message))
        };
        let longer_lived_value;
        let snippet = Snippet {
            title: Some(Annotation {
                label: Some(&short_message),
                id: None,
                annotation_type,
            }),
            footer: if issue.replacements.is_empty() {
                Vec::new()
            } else {
                longer_lived_value = format!(
                    "try replacing the text with \"{}\"",
                    issue.replacements[0].value
                );
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
                fold: true,
                annotations: vec![SourceAnnotation {
                    label: &message,
                    annotation_type,
                    range: (issue.offset, issue.offset + issue.length),
                }],
            }],
            opt: FormatOptions {
                color: true,
                ..Default::default()
            },
        };
        println!("{}", DisplayList::from(snippet));
    }
}
