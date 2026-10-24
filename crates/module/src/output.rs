use serde::Serialize;

/// One line of Waybar `custom/*` JSON output (`"return-type": "json"`).
///
/// `text`, `alt` and `tooltip` are Pango markup unless the module sets
/// `"escape": true`. See `man 5 waybar-custom`.
#[derive(Debug, Default, Serialize)]
pub struct Output {
    /// Replaces `{text}` in `format`.
    pub text: String,
    /// Replaces `{alt}` and selects the icon from an object-valued `format-icons`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    /// CSS classes, styled as `#custom-<name>.<class>`.
    pub class: Vec<String>,
    /// 0–100. Replaces `{percentage}` and indexes an array-valued `format-icons`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage: Option<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_on_one_line_without_empty_fields() {
        let out = Output { text: "1".into(), class: vec!["empty".into()], ..Default::default() };
        assert_eq!(serde_json::to_string(&out).unwrap(), r#"{"text":"1","class":["empty"]}"#);
    }
}
