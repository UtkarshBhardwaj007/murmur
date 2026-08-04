//! Transcript post-processing: turn raw engine output into text that is
//! pleasant to paste.

/// Clean a raw transcript:
/// 1. strip engine noise markers like `[BLANK_AUDIO]` or `(inaudible)`,
/// 2. strip SentencePiece word-boundary markers (`▁`),
/// 3. drop a trailing partial token (a word cut off with `-` or `…`),
/// 4. collapse whitespace and trim,
/// 5. capitalize the first letter.
pub fn post_process(raw: &str) -> String {
    let no_markers = strip_noise_markers(raw);
    let no_sp = no_markers.replace('▁', " ");

    let mut words: Vec<&str> = no_sp.split_whitespace().collect();
    if let Some(last) = words.last() {
        if is_partial_token(last) {
            words.pop();
        }
    }
    let mut text = words.join(" ");

    if let Some(first) = text.chars().next() {
        if first.is_lowercase() {
            let upper: String = first.to_uppercase().collect();
            text.replace_range(..first.len_utf8(), &upper);
        }
    }
    text
}

/// Remove `[...]` and `(...)` segments that are engine annotations rather
/// than speech — `[BLANK_AUDIO]`, `[MUSIC]`, `(inaudible)`, `(speaking in
/// foreign language)` and friends. Real parenthesised speech is kept.
fn strip_noise_markers(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let close = match c {
            '[' => Some(']'),
            '(' => Some(')'),
            _ => None,
        };
        let Some(close) = close else {
            out.push(c);
            continue;
        };
        match input[i + 1..].find(close) {
            Some(rel_end) => {
                let inner = &input[i + 1..i + 1 + rel_end];
                if is_noise_annotation(c == '[', inner) {
                    // Skip everything up to and including the closing char.
                    for (_, c2) in chars.by_ref() {
                        if c2 == close {
                            break;
                        }
                    }
                } else {
                    out.push(c);
                }
            }
            None => out.push(c),
        }
    }
    out
}

fn is_noise_annotation(bracketed: bool, inner: &str) -> bool {
    // Whisper-style `[BLANK_AUDIO]`: all caps/underscores/spaces in brackets.
    if bracketed
        && !inner.is_empty()
        && inner
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c == ' ')
    {
        return true;
    }
    // Known parenthesised annotations.
    const NOISE: &[&str] = &[
        "inaudible",
        "silence",
        "music",
        "laughs",
        "laughter",
        "applause",
        "noise",
        "coughs",
        "speaking in foreign language",
    ];
    let lower = inner.trim().to_lowercase();
    NOISE.contains(&lower.as_str())
}

/// A trailing token the engine likely cut off mid-word.
fn is_partial_token(token: &str) -> bool {
    token.ends_with('-')
        || token.ends_with('…')
        || token == "▁"
        || token.chars().all(|c| c == '.') && token.len() > 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_collapses_whitespace() {
        assert_eq!(post_process("  hello   world  "), "Hello world");
    }

    #[test]
    fn capitalizes_first_letter() {
        assert_eq!(post_process("this is a test."), "This is a test.");
    }

    #[test]
    fn leaves_already_capitalized_text_alone() {
        assert_eq!(post_process("Hello there."), "Hello there.");
    }

    #[test]
    fn strips_whisper_blank_audio_marker() {
        assert_eq!(post_process("[BLANK_AUDIO]"), "");
        assert_eq!(post_process("hello [BLANK_AUDIO] world"), "Hello world");
        assert_eq!(post_process(" [MUSIC] okay"), "Okay");
    }

    #[test]
    fn strips_parenthesised_noise_but_keeps_real_parentheses() {
        assert_eq!(post_process("well (inaudible) then"), "Well then");
        assert_eq!(
            post_process("add two (or three) eggs"),
            "Add two (or three) eggs"
        );
    }

    #[test]
    fn strips_sentencepiece_markers() {
        assert_eq!(post_process("▁hello▁world"), "Hello world");
    }

    #[test]
    fn drops_trailing_partial_token() {
        assert_eq!(post_process("see you tomo-"), "See you");
        assert_eq!(post_process("and then…"), "And");
        assert_eq!(post_process("wait for it ..."), "Wait for it");
    }

    #[test]
    fn empty_and_silence_inputs_give_empty_output() {
        assert_eq!(post_process(""), "");
        assert_eq!(post_process("   "), "");
        assert_eq!(post_process("[BLANK_AUDIO] [SILENCE]"), "");
    }

    #[test]
    fn unicode_first_letter_is_capitalized() {
        assert_eq!(post_process("émile was here"), "Émile was here");
    }
}
