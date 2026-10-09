//! Pasting image paths into Claude Code and reading back whether it attached them.

use std::path::PathBuf;

use serde::Serialize;

use crate::error::AppError;

/// How Claude Code starts the placeholder it shows for each attached image (`[Image #1]`).
pub const IMAGE_PLACEHOLDER: &str = "[Image";
const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenState {
    pub bracketed_paste: bool,
    pub image_placeholders: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SubmitOutcome {
    Submitted,
    NotRecognized { recognized: usize },
}

/// One bracketed paste with the paths separated by spaces, the way a file drop arrives.
/// Control characters are refused: they could end the paste early and type keys.
pub fn bracketed_paste(paths: &[PathBuf]) -> Result<Vec<u8>, AppError> {
    if paths.is_empty() {
        return Err(AppError::Invalid("nenhuma imagem para enviar".to_owned()));
    }
    let mut texts = Vec::with_capacity(paths.len());
    for path in paths {
        let text = path
            .to_str()
            .filter(|text| path.is_absolute() && !text.chars().any(char::is_control))
            .ok_or_else(|| AppError::Invalid(format!("caminho de imagem inválido: {}", path.display())))?;
        texts.push(text);
    }
    let mut bytes = PASTE_START.to_vec();
    bytes.extend_from_slice(texts.join(" ").as_bytes());
    bytes.extend_from_slice(PASTE_END);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn paths_are_pasted_as_one_bracketed_paste() {
        let paths = [PathBuf::from("/Users/dev/Library/Application Support/ClaudeCodeManager/attachments/a.png"), PathBuf::from("/tmp/Área/b.png")];
        let bytes = bracketed_paste(&paths).unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "\x1b[200~/Users/dev/Library/Application Support/ClaudeCodeManager/attachments/a.png /tmp/Área/b.png\x1b[201~"
        );
    }

    #[test]
    fn relative_paths_control_characters_and_empty_lists_are_refused() {
        assert!(bracketed_paste(&[]).is_err());
        assert!(bracketed_paste(&[PathBuf::from("a.png")]).is_err());
        assert!(bracketed_paste(&[PathBuf::from("/tmp/a\x1b[201~\r.png")]).is_err());
    }

    #[test]
    fn outcomes_serialize_for_the_frontend() {
        assert_eq!(serde_json::to_string(&SubmitOutcome::Submitted).unwrap(), r#"{"kind":"submitted"}"#);
        assert_eq!(serde_json::to_string(&SubmitOutcome::NotRecognized { recognized: 1 }).unwrap(), r#"{"kind":"notRecognized","recognized":1}"#);
    }
}
