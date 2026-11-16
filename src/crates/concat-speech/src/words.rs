// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer Abdullah and Concat contributors

//! Whisper's tokens as words with times.
//!
//! Whisper writes in tokens: pieces of words, and sometimes pieces of a
//! letter - "ñ" and "á" are two bytes, and a token may end between them.
//! With token timestamps on it also says when it heard each one. A caption
//! that lights the word being said wants neither pieces nor bytes: it
//! wants "¿Cómo" from 1.20 to 1.48.
//!
//! [`words`] puts the pieces back together. A token whose text begins with
//! a space begins a word, as whisper's vocabulary has it for every
//! language that writes spaces; everything up to the next such token is
//! the same word, punctuation included, so a comma or a question mark
//! stays on the word it follows and an opening "¿" on the one it leads.
//! The bytes are joined before they are read as text, so a letter split
//! across two tokens is whole again. A word runs from its first token's
//! start to its last one's end.
//!
//! Nothing here calls whisper: it takes the tokens as plain values, so it
//! is tested without a model.

/// One of whisper's tokens, as much of it as a word needs.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    /// The token's text, as bytes: not always whole UTF-8 on its own.
    pub bytes: Vec<u8>,
    /// Where whisper heard it begin, in seconds.
    pub start: f64,
    /// Where it heard it end.
    pub end: f64,
}

/// One word: its letters and punctuation, and when it was said.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Word {
    /// Seconds, on the clock the tokens came in on.
    pub start: f64,
    /// Seconds; never before `start`.
    pub end: f64,
    /// The word, with no space around it.
    pub text: String,
}

/// The words `pieces` spell, in order, each from its first piece's start
/// to its last one's end. Times are made to run forwards: a word never
/// starts before the one ahead of it did, nor ends before it starts.
pub fn words(pieces: &[Piece]) -> Vec<Word> {
    let mut out: Vec<Word> = Vec::new();
    let mut held: Vec<u8> = Vec::new();
    let mut span: Option<(f64, f64)> = None;
    let flush = |held: &mut Vec<u8>, span: &mut Option<(f64, f64)>, out: &mut Vec<Word>| {
        let text = String::from_utf8_lossy(held).trim().to_owned();
        held.clear();
        let Some((start, end)) = span.take() else {
            return;
        };
        if text.is_empty() {
            return;
        }
        let floor = out.last().map_or(f64::NEG_INFINITY, |word| word.start);
        let start = if start.is_finite() {
            start.max(floor)
        } else {
            floor.max(0.0)
        };
        let end = if end.is_finite() {
            end.max(start)
        } else {
            start
        };
        // Punctuation that stands alone - a dash between two spaces, a
        // quote whisper spaced off - is no word to light: it joins the
        // word before it, as a reader would take it.
        let alone = !text.chars().any(char::is_alphanumeric);
        match out.last_mut() {
            Some(last) if alone => {
                last.text.push_str(&text);
                last.end = last.end.max(end);
            }
            _ => out.push(Word { start, end, text }),
        }
    };
    for piece in pieces {
        // A space opens a word. So does the first piece, whatever it is.
        if piece.bytes.first() == Some(&b' ') && !held.is_empty() {
            flush(&mut held, &mut span, &mut out);
        }
        held.extend_from_slice(&piece.bytes);
        span = Some(match span {
            Some((start, end)) => (start.min(piece.start), end.max(piece.end)),
            None => (piece.start, piece.end),
        });
    }
    flush(&mut held, &mut span, &mut out);
    out
}

/// `words` held inside `start..end` and kept apart: none begins before
/// `start` or ends after `end`, and none runs on past where the next
/// begins. Whisper's token times wander a little past the phrase they
/// belong to, and a word that outlasts its phrase would be lit over the
/// next caption's.
pub fn held_within(mut words: Vec<Word>, start: f64, end: f64) -> Vec<Word> {
    let end = end.max(start);
    for word in &mut words {
        word.start = word.start.clamp(start, end);
        word.end = word.end.clamp(word.start, end);
    }
    for index in 1..words.len() {
        let next = words[index].start;
        let before = &mut words[index - 1];
        before.end = before.end.min(next).max(before.start);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(text: &str, start: f64, end: f64) -> Piece {
        Piece {
            bytes: text.as_bytes().to_vec(),
            start,
            end,
        }
    }

    fn spelled(words: &[Word]) -> Vec<&str> {
        words.iter().map(|word| word.text.as_str()).collect()
    }

    #[test]
    fn a_space_begins_a_word_and_punctuation_stays_on_it() {
        let heard = words(&[
            piece(" Hola", 0.0, 0.3),
            piece(",", 0.3, 0.35),
            piece(" ¿", 0.5, 0.5),
            piece("c", 0.5, 0.55),
            piece("ómo", 0.55, 0.8),
            piece(" est", 0.8, 1.0),
            piece("ás", 1.0, 1.2),
            piece("?", 1.2, 1.25),
        ]);
        assert_eq!(spelled(&heard), vec!["Hola,", "¿cómo", "estás?"]);
        assert_eq!((heard[0].start, heard[0].end), (0.0, 0.35));
        assert_eq!((heard[1].start, heard[1].end), (0.5, 0.8));
        assert_eq!((heard[2].start, heard[2].end), (0.8, 1.25));
    }

    #[test]
    fn a_letter_split_across_two_tokens_is_whole_again() {
        // "ñ" is 0xC3 0xB1; whisper's byte-level vocabulary can end a
        // token between the two.
        let heard = words(&[
            Piece {
                bytes: vec![b' ', b'a', 0xC3],
                start: 0.0,
                end: 0.2,
            },
            Piece {
                bytes: vec![0xB1, b'o'],
                start: 0.2,
                end: 0.4,
            },
        ]);
        assert_eq!(spelled(&heard), vec!["año"]);
        assert_eq!((heard[0].start, heard[0].end), (0.0, 0.4));
    }

    #[test]
    fn the_first_piece_begins_a_word_without_a_space() {
        let heard = words(&[piece("Bueno", 0.1, 0.4), piece(" ya", 0.4, 0.6)]);
        assert_eq!(spelled(&heard), vec!["Bueno", "ya"]);
    }

    #[test]
    fn punctuation_alone_joins_the_word_before() {
        let heard = words(&[
            piece(" sí", 0.0, 0.2),
            piece(" -", 0.2, 0.3),
            piece(" claro", 0.4, 0.8),
            piece(" ...", 0.8, 0.9),
        ]);
        assert_eq!(spelled(&heard), vec!["sí-", "claro..."]);
        assert_eq!(heard[0].end, 0.3);
    }

    #[test]
    fn times_run_forwards_whatever_whisper_said() {
        let heard = words(&[
            piece(" uno", 1.0, 1.4),
            piece(" dos", 0.8, 0.7),
            piece(" tres", f64::NAN, 2.0),
        ]);
        assert_eq!((heard[1].start, heard[1].end), (1.0, 1.0));
        assert_eq!(heard[2].start, 1.0);
        assert_eq!(heard[2].end, 2.0);
    }

    #[test]
    fn nothing_and_blanks_spell_no_words() {
        assert!(words(&[]).is_empty());
        assert!(words(&[piece(" ", 0.0, 0.1), piece("  ", 0.1, 0.2)]).is_empty());
    }

    #[test]
    fn words_are_held_inside_their_phrase_and_apart() {
        let held = held_within(
            vec![
                Word {
                    start: 0.8,
                    end: 1.6,
                    text: "uno".to_owned(),
                },
                Word {
                    start: 1.4,
                    end: 2.9,
                    text: "dos".to_owned(),
                },
            ],
            1.0,
            2.5,
        );
        assert_eq!((held[0].start, held[0].end), (1.0, 1.4));
        assert_eq!((held[1].start, held[1].end), (1.4, 2.5));
    }
}
