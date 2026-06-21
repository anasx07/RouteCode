use similar::{ChangeTag, TextDiff};

pub fn generate(old: &str, new: &str) -> String {
    let mut diff_str = String::new();
    let diff = TextDiff::from_lines(old, new);

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        diff_str.push_str(&format!("{}{}", sign, change));
    }
    diff_str
}
