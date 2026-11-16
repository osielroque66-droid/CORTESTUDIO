// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer Abdullah and Concat contributors

//! Captions from spoken words: how a transcript is cut into the lines a
//! viewer reads, and those lines written out as WebVTT, Advanced
//! SubStation Alpha and plain text.
//!
//! A transcriber hears phrases; a reel wants one word at a time, a pair,
//! a short burst, or a sentence. [`group`] cuts the same words each way
//! ([`Grouping`]), and every line it makes keeps its words' own times, so
//! the word being said can be lit whichever way the lines were cut (see
//! [`crate::karaoke`]).
//!
//! SubRip stays in [`crate::srt`], which also reads it. The writers here
//! take the same cues with their words' times beside them, where a format
//! has a use for them: ASS writes them as `\k` karaoke tags, which a
//! player sweeps through as the line is said.

use crate::karaoke::{self, WordTime};
use crate::model::{ClipKind, Timeline};

/// One word as it was heard: its letters with their punctuation, and
/// when, in seconds on whatever clock the caller keeps.
#[derive(Clone, Debug, PartialEq)]
pub struct SpokenWord {
    /// Where it begins.
    pub start: f64,
    /// Where it ends.
    pub end: f64,
    /// The word.
    pub text: String,
}

/// One phrase as it was heard, with its words where they were timed.
#[derive(Clone, Debug, PartialEq)]
pub struct Spoken {
    /// Where it begins.
    pub start: f64,
    /// Where it ends.
    pub end: f64,
    /// What was said.
    pub text: String,
    /// Its words, in order. Empty when only the phrase was timed, and the
    /// words' times are then the phrase's, shared by length.
    pub words: Vec<SpokenWord>,
}

impl Spoken {
    /// The words with times: the ones heard when they are the phrase's
    /// own words, and otherwise the phrase's span shared among its words
    /// by their lengths. A transcript tidied after it was timed - a loop
    /// said once, a case changed - no longer has a time for each word.
    pub fn timed_words(&self) -> Vec<SpokenWord> {
        let written: Vec<&str> = self.text.split_whitespace().collect();
        if !self.words.is_empty()
            && self.words.len() == written.len()
            && self
                .words
                .iter()
                .zip(&written)
                .all(|(word, text)| word.text.trim() == *text)
        {
            return self.words.clone();
        }
        let lengths: Vec<usize> = written.iter().map(|word| word.chars().count()).collect();
        karaoke::spread(&lengths, self.start, self.end.max(self.start))
            .into_iter()
            .zip(written)
            .map(|(time, text)| SpokenWord {
                start: time.at,
                end: time.until,
                text: text.to_owned(),
            })
            .collect()
    }
}

/// How a transcript is cut into captions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Grouping {
    /// The phrases as they were heard, a long one wrapped at its words.
    #[default]
    Phrase,
    /// One word at a time.
    Word,
    /// Two words at a time.
    Pair,
    /// Short bursts of three to five words, ended early by a sentence's
    /// end or a pause.
    Burst,
    /// Whole sentences, a long one wrapped at its words.
    Sentence,
}

/// One caption: its words, when it shows, and when each word is said.
#[derive(Clone, Debug, PartialEq)]
pub struct Caption {
    /// Seconds where it shows, on the clock the words came in on.
    pub start: f64,
    /// Seconds where it goes.
    pub end: f64,
    /// The words, a space between them.
    pub text: String,
    /// One time per word of `text`, on the same clock as `start`.
    pub words: Vec<WordTime>,
}

impl Caption {
    /// The words' times with the caption's own start as zero: what a
    /// title clip made of it keeps (see [`crate::karaoke::Karaoke`]).
    pub fn local_words(&self) -> Vec<WordTime> {
        self.words
            .iter()
            .map(|word| WordTime {
                at: word.at - self.start,
                until: word.until - self.start,
            })
            .collect()
    }
}

/// The longest a caption line runs before it wraps: what two lines of a
/// broadcast subtitle hold, and what a reader takes in at a glance.
pub const LINE_CHARS: usize = 42;
/// A pause this long between two words ends a burst: the speaker stopped,
/// and the words after belong to the next thought.
const PAUSE: f64 = 0.6;
/// The longest a sentence is held as one caption, in characters, before
/// it is wrapped at its words.
const SENTENCE_CHARS: usize = 84;

/// Whether `word` ends a sentence: a full stop, a question or exclamation
/// mark or an ellipsis at its end, a closing quote or bracket allowed
/// after. "3.5" does not, and neither does a comma.
fn ends_sentence(word: &str) -> bool {
    let trimmed = word.trim_end_matches(['"', '\'', '»', '”', '’', ')', ']']);
    trimmed.ends_with(['.', '!', '?', '…', '。', '！', '？'])
}

/// Whether `word` ends a clause: a sentence's end, or a comma, semicolon
/// or colon.
fn ends_clause(word: &str) -> bool {
    ends_sentence(word) || word.ends_with([',', ';', ':'])
}

/// Cuts a transcript into captions. Every word lands in exactly one
/// caption, in order, with its own times; a caption shows from its first
/// word to its last one's end.
pub fn group(spoken: &[Spoken], grouping: Grouping) -> Vec<Caption> {
    let mut captions = Vec::new();
    match grouping {
        Grouping::Phrase => {
            for phrase in spoken {
                wrap(&phrase.timed_words(), LINE_CHARS, &mut captions);
            }
        }
        Grouping::Sentence => {
            let words: Vec<SpokenWord> = spoken.iter().flat_map(Spoken::timed_words).collect();
            let mut sentence: Vec<SpokenWord> = Vec::new();
            for word in words {
                let ends = ends_sentence(&word.text);
                sentence.push(word);
                if ends {
                    wrap(&sentence, SENTENCE_CHARS, &mut captions);
                    sentence.clear();
                }
            }
            wrap(&sentence, SENTENCE_CHARS, &mut captions);
        }
        Grouping::Word | Grouping::Pair | Grouping::Burst => {
            let most = match grouping {
                Grouping::Word => 1,
                Grouping::Pair => 2,
                _ => 5,
            };
            let words: Vec<SpokenWord> = spoken.iter().flat_map(Spoken::timed_words).collect();
            let mut held: Vec<SpokenWord> = Vec::new();
            for (index, word) in words.iter().enumerate() {
                held.push(word.clone());
                let next = words.get(index + 1);
                let pause = next.is_some_and(|next| next.start - word.end >= PAUSE);
                let full = held.len() >= most;
                // A burst of three or more ends at a clause; any burst
                // ends at a sentence. A pair or a word ends only when
                // full, at a sentence's end, or at a pause.
                let natural = ends_sentence(&word.text)
                    || (grouping == Grouping::Burst && held.len() >= 3 && ends_clause(&word.text));
                if full || pause || natural || next.is_none() {
                    push(&held, &mut captions);
                    held.clear();
                }
            }
        }
    }
    captions
}

/// `words` as captions of at most `chars` characters, cut between words -
/// at the clause nearest the middle when the line has one, so a long
/// line breaks where a reader would breathe.
fn wrap(words: &[SpokenWord], chars: usize, out: &mut Vec<Caption>) {
    if words.is_empty() {
        return;
    }
    let length: usize = words
        .iter()
        .map(|word| word.text.chars().count())
        .sum::<usize>()
        + words.len()
        - 1;
    if length <= chars || words.len() == 1 {
        push(words, out);
        return;
    }
    // The cut nearest the middle, a clause's end preferred over a plain
    // gap by as much as a quarter of the line.
    let mut best = (1, f64::INFINITY);
    let mut before = 0usize;
    for (index, word) in words.iter().enumerate().take(words.len() - 1) {
        before += word.text.chars().count() + 1;
        let off = (before as f64 - length as f64 / 2.0).abs();
        let cost = if ends_clause(&word.text) {
            off - length as f64 / 4.0
        } else {
            off
        };
        if cost < best.1 {
            best = (index + 1, cost);
        }
    }
    wrap(&words[..best.0], chars, out);
    wrap(&words[best.0..], chars, out);
}

fn push(words: &[SpokenWord], out: &mut Vec<Caption>) {
    let (Some(first), Some(last)) = (words.first(), words.last()) else {
        return;
    };
    let text = words
        .iter()
        .map(|word| word.text.trim())
        .collect::<Vec<_>>()
        .join(" ");
    if text.trim().is_empty() {
        return;
    }
    out.push(Caption {
        start: first.start,
        end: last.end.max(first.start),
        text,
        words: words
            .iter()
            .map(|word| WordTime {
                at: word.start,
                until: word.end.max(word.start),
            })
            .collect(),
    });
}

/// The captions the title clips on `track_id` make - on every lane, for
/// None - in the order they play, each with its words' times on the
/// timeline's clock: the times a karaoke title keeps, or the title's span
/// shared among its words when it keeps none.
pub fn title_captions(timeline: &Timeline, track_id: Option<&str>) -> Vec<Caption> {
    let mut captions: Vec<Caption> = timeline
        .clips
        .iter()
        .filter(|clip| clip.kind == ClipKind::Text)
        .filter(|clip| track_id.is_none_or(|id| clip.track_id == id))
        .filter_map(|clip| {
            let text = clip.text.as_ref()?;
            let (start, end) = (clip.start, clip.start + clip.duration);
            let kept = text
                .karaoke
                .as_ref()
                .map(|karaoke| karaoke.fitted(&text.content))
                .filter(|words| !words.is_empty());
            let words = match kept {
                Some(words) => words
                    .into_iter()
                    .map(|word| {
                        // From the clip's source clock onto the
                        // timeline's, held inside the clip.
                        let at = (start + word.at - clip.source_start).clamp(start, end);
                        let until = (start + word.until - clip.source_start).clamp(at, end);
                        WordTime { at, until }
                    })
                    .collect(),
                None => {
                    let lengths: Vec<usize> = karaoke::word_ranges(&text.content)
                        .into_iter()
                        .map(|(from, to)| to - from)
                        .collect();
                    karaoke::spread(&lengths, start, end)
                }
            };
            Some(Caption {
                start,
                end,
                text: text.content.clone(),
                words,
            })
        })
        .collect();
    captions.sort_by(|a, b| a.start.total_cmp(&b.start));
    captions
}

/// `HH:MM:SS.mmm`, to the nearest millisecond; below zero is zero.
fn vtt_stamp(seconds: f64) -> String {
    let millis = (seconds.max(0.0) * 1_000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        millis / 3_600_000,
        millis / 60_000 % 60,
        millis / 1_000 % 60,
        millis % 1_000
    )
}

/// Writes captions as WebVTT. `-->` inside a caption's words would read
/// as a timing line, and `&` and `<` as markup, so they are escaped; a
/// blank line would end the cue, so blank lines are left out.
pub fn write_vtt(captions: &[Caption]) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for (index, caption) in captions.iter().enumerate() {
        out.push_str(&format!(
            "{}\n{} --> {}\n",
            index + 1,
            vtt_stamp(caption.start),
            vtt_stamp(caption.end)
        ));
        for line in caption.text.lines().map(str::trim_end) {
            if !line.is_empty() {
                out.push_str(
                    &line
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;"),
                );
                out.push('\n');
            }
        }
        out.push('\n');
    }
    out
}

/// Writes captions as plain text: one caption a line, in order, with no
/// times. A caption's own line breaks become spaces.
pub fn write_txt(captions: &[Caption]) -> String {
    let mut out = String::new();
    for caption in captions {
        let line = caption
            .text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !line.is_empty() {
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

/// How an ASS file's one style looks. Colours are `#rrggbb`.
#[derive(Clone, Debug, PartialEq)]
pub struct AssLook {
    /// The frame the positions and sizes are for, in pixels.
    pub frame: (u32, u32),
    /// The face's family name, without quotes.
    pub font: String,
    /// The text's height in frame pixels.
    pub size: f64,
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
    /// The words' colour.
    pub color: String,
    /// The colour a word takes as it is said, when the captions carry
    /// their words' times; None writes no karaoke tags.
    pub spoken: Option<String>,
    /// The outline's colour.
    pub outline: String,
    /// The outline's thickness in frame pixels.
    pub outline_width: f64,
    /// Whether the words cast a shadow.
    pub shadow: bool,
    /// Where the block sits: 2 bottom centre, 5 middle, 8 top centre, as
    /// a numeric keypad is laid out.
    pub alignment: u8,
    /// The margin from the frame's edge the alignment names, in pixels.
    pub margin_v: u32,
}

impl Default for AssLook {
    fn default() -> Self {
        Self {
            frame: (1080, 1920),
            font: "Hanken Grotesk".to_owned(),
            size: 72.0,
            bold: true,
            italic: false,
            color: "#ffffff".to_owned(),
            spoken: None,
            outline: "#000000".to_owned(),
            outline_width: 4.0,
            shadow: true,
            alignment: 2,
            margin_v: 160,
        }
    }
}

/// `#rrggbb` as ASS writes a colour: `&H00BBGGRR`. Anything else is
/// white.
fn ass_colour(hex: &str) -> String {
    let digits = hex.trim().trim_start_matches('#');
    let part = |at: usize| {
        digits
            .get(at..at + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
    };
    match (part(0), part(2), part(4)) {
        (Some(r), Some(g), Some(b)) if digits.len() >= 6 => format!("&H00{b:02X}{g:02X}{r:02X}"),
        _ => "&H00FFFFFF".to_owned(),
    }
}

/// `H:MM:SS.cc`, to the nearest centisecond, as ASS times an event.
fn ass_stamp(seconds: f64) -> String {
    let centis = (seconds.max(0.0) * 100.0).round() as u64;
    format!(
        "{}:{:02}:{:02}.{:02}",
        centis / 360_000,
        centis / 6_000 % 60,
        centis / 100 % 60,
        centis % 100
    )
}

/// Writes captions as Advanced SubStation Alpha, in one style. With
/// `look.spoken` set and a caption's words timed, each word is written
/// behind a `\k` tag holding the centiseconds until the next, and a
/// player fills the line in the spoken colour as it is said.
pub fn write_ass(captions: &[Caption], look: &AssLook) -> String {
    let flag = |on: bool| if on { -1 } else { 0 };
    // With karaoke, ASS fills from the secondary colour to the primary:
    // the spoken colour is the primary, the unspoken one the secondary.
    let (primary, secondary) = match &look.spoken {
        Some(spoken) => (ass_colour(spoken), ass_colour(&look.color)),
        None => (ass_colour(&look.color), ass_colour(&look.color)),
    };
    let mut out = format!(
        "[Script Info]\n\
         ScriptType: v4.00+\n\
         PlayResX: {}\n\
         PlayResY: {}\n\
         WrapStyle: 0\n\
         ScaledBorderAndShadow: yes\n\
         \n\
         [V4+ Styles]\n\
         Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, \
         Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, \
         Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
         Style: Default,{},{:.0},{},{},{},&H80000000,{},{},0,0,100,100,0,0,1,{:.1},{},{},40,40,{},1\n\
         \n\
         [Events]\n\
         Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        look.frame.0,
        look.frame.1,
        look.font.replace(',', " "),
        look.size,
        primary,
        secondary,
        ass_colour(&look.outline),
        flag(look.bold),
        flag(look.italic),
        look.outline_width.max(0.0),
        if look.shadow { 2 } else { 0 },
        look.alignment.clamp(1, 9),
        look.margin_v,
    );
    for caption in captions {
        let written: Vec<&str> = caption.text.split_whitespace().collect();
        if written.is_empty() {
            continue;
        }
        // Braces open an override block and a backslash an escape; a
        // caption's own are written as their full-width look-alikes.
        let clean = |word: &str| {
            word.replace('{', "｛")
                .replace('}', "｝")
                .replace('\\', "＼")
        };
        let text = if look.spoken.is_some() && caption.words.len() == written.len() {
            let mut line = String::new();
            // The wait before the first word, so the fill starts when the
            // word does and not when the line shows.
            let lead = ((caption.words[0].at - caption.start).max(0.0) * 100.0).round() as u64;
            if lead > 0 {
                line.push_str(&format!("{{\\k{lead}}}"));
            }
            for (index, (word, time)) in written.iter().zip(&caption.words).enumerate() {
                let next = caption
                    .words
                    .get(index + 1)
                    .map_or(caption.end, |next| next.at);
                let centis = ((next - time.at).max(0.0) * 100.0).round() as u64;
                if index > 0 {
                    line.push(' ');
                }
                line.push_str(&format!("{{\\k{centis}}}{}", clean(word)));
            }
            line
        } else {
            caption
                .text
                .lines()
                .map(|line| clean(line.trim()))
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join("\\N")
        };
        out.push_str(&format!(
            "Dialogue: 0,{},{},Default,,0,0,0,,{}\n",
            ass_stamp(caption.start),
            ass_stamp(caption.end),
            text
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(text: &str, start: f64, end: f64) -> SpokenWord {
        SpokenWord {
            start,
            end,
            text: text.to_owned(),
        }
    }

    /// "Hola a todos, bienvenidos al canal. Hoy vamos a editar un video."
    /// with a pause before "Hoy".
    fn heard() -> Vec<Spoken> {
        let first = vec![
            word("Hola", 0.0, 0.3),
            word("a", 0.3, 0.4),
            word("todos,", 0.4, 0.8),
            word("bienvenidos", 0.9, 1.5),
            word("al", 1.5, 1.6),
            word("canal.", 1.6, 2.0),
        ];
        let second = vec![
            word("Hoy", 3.0, 3.2),
            word("vamos", 3.2, 3.5),
            word("a", 3.5, 3.6),
            word("editar", 3.6, 4.0),
            word("un", 4.0, 4.1),
            word("video.", 4.1, 4.6),
        ];
        vec![
            Spoken {
                start: 0.0,
                end: 2.0,
                text: "Hola a todos, bienvenidos al canal.".to_owned(),
                words: first,
            },
            Spoken {
                start: 3.0,
                end: 4.6,
                text: "Hoy vamos a editar un video.".to_owned(),
                words: second,
            },
        ]
    }

    fn texts(captions: &[Caption]) -> Vec<&str> {
        captions
            .iter()
            .map(|caption| caption.text.as_str())
            .collect()
    }

    /// Every word, once, in order, whichever way the lines are cut.
    fn all_words(captions: &[Caption]) -> String {
        texts(captions).join(" ")
    }

    const WHOLE: &str = "Hola a todos, bienvenidos al canal. Hoy vamos a editar un video.";

    #[test]
    fn phrases_stay_as_heard() {
        let captions = group(&heard(), Grouping::Phrase);
        assert_eq!(
            texts(&captions),
            vec![
                "Hola a todos, bienvenidos al canal.",
                "Hoy vamos a editar un video."
            ]
        );
        assert_eq!((captions[0].start, captions[0].end), (0.0, 2.0));
        assert_eq!(captions[0].words.len(), 6);
        assert_eq!(captions[1].local_words()[0].at, 0.0);
    }

    #[test]
    fn one_word_at_a_time_each_with_its_own_span() {
        let captions = group(&heard(), Grouping::Word);
        assert_eq!(captions.len(), 12);
        assert_eq!(all_words(&captions), WHOLE);
        assert_eq!(captions[3].text, "bienvenidos");
        assert_eq!((captions[3].start, captions[3].end), (0.9, 1.5));
    }

    #[test]
    fn pairs_end_at_a_sentence_and_a_pause() {
        let captions = group(&heard(), Grouping::Pair);
        assert_eq!(
            texts(&captions),
            vec![
                "Hola a",
                "todos, bienvenidos",
                "al canal.",
                "Hoy vamos",
                "a editar",
                "un video."
            ]
        );
        assert_eq!(all_words(&captions), WHOLE);
    }

    #[test]
    fn bursts_break_where_the_speaker_does() {
        let captions = group(&heard(), Grouping::Burst);
        assert_eq!(
            texts(&captions),
            vec![
                "Hola a todos,",
                "bienvenidos al canal.",
                "Hoy vamos a editar un",
                "video."
            ]
        );
        assert!(captions.iter().all(|caption| caption.words.len() <= 5));
        assert_eq!(all_words(&captions), WHOLE);
    }

    #[test]
    fn sentences_join_phrases_and_wrap_long_ones() {
        let mut spoken = heard();
        // One sentence heard as two phrases.
        spoken[0].text = "Hola a todos, bienvenidos al canal".to_owned();
        spoken[0].words[5].text = "canal".to_owned();
        let captions = group(&spoken, Grouping::Sentence);
        assert_eq!(captions.len(), 1);
        assert_eq!(captions[0].words.len(), 12);
        assert_eq!((captions[0].start, captions[0].end), (0.0, 4.6));

        let long: Vec<SpokenWord> = (0..40)
            .map(|index| word("palabra", f64::from(index), f64::from(index) + 0.5))
            .collect();
        let captions = group(
            &[Spoken {
                start: 0.0,
                end: 40.0,
                text: vec!["palabra"; 40].join(" "),
                words: long,
            }],
            Grouping::Sentence,
        );
        assert!(captions.len() > 1);
        assert!(
            captions
                .iter()
                .all(|caption| caption.text.chars().count() <= SENTENCE_CHARS)
        );
        assert_eq!(
            captions
                .iter()
                .map(|caption| caption.words.len())
                .sum::<usize>(),
            40
        );
    }

    #[test]
    fn a_phrase_with_no_timed_words_shares_its_span_by_length() {
        let spoken = Spoken {
            start: 10.0,
            end: 12.0,
            text: "sí claro".to_owned(),
            words: Vec::new(),
        };
        let words = spoken.timed_words();
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].start, 10.0);
        assert!((words[0].end - 10.0 - 2.0 * 2.0 / 7.0).abs() < 1e-9);
        assert!((words[1].end - 12.0).abs() < 1e-9);

        // Words that are no longer the phrase's are not trusted either.
        let tidied = Spoken {
            words: vec![word("si", 10.0, 10.1), word("si", 10.1, 10.2)],
            ..spoken
        };
        assert_eq!(tidied.timed_words()[1].text, "claro");
    }

    #[test]
    fn a_long_phrase_wraps_at_its_clause() {
        let text = "Cuando termines de grabar, revisa bien el audio antes de exportar";
        let spoken = Spoken {
            start: 0.0,
            end: 6.0,
            text: text.to_owned(),
            words: Vec::new(),
        };
        let captions = group(&[spoken], Grouping::Phrase);
        assert_eq!(
            texts(&captions),
            vec![
                "Cuando termines de grabar,",
                "revisa bien el audio antes de exportar"
            ]
        );
        assert_eq!(captions[0].end, captions[1].start);
    }

    #[test]
    fn spanish_punctuation_ends_sentences() {
        assert!(ends_sentence("¿verdad?"));
        assert!(ends_sentence("¡claro!»"));
        assert!(ends_sentence("bueno…"));
        assert!(!ends_sentence("3.5"));
        assert!(!ends_sentence("todos,"));
        assert!(ends_clause("todos,"));
    }

    #[test]
    fn vtt_has_its_header_dots_and_escapes() {
        let captions = vec![Caption {
            start: 1.5,
            end: 3661.25,
            text: "Tom & Jerry <3\n\nsegunda línea".to_owned(),
            words: Vec::new(),
        }];
        assert_eq!(
            write_vtt(&captions),
            "WEBVTT\n\n1\n00:00:01.500 --> 01:01:01.250\nTom &amp; Jerry &lt;3\nsegunda línea\n\n"
        );
    }

    #[test]
    fn txt_is_a_line_per_caption() {
        let captions = group(&heard(), Grouping::Phrase);
        assert_eq!(
            write_txt(&captions),
            "Hola a todos, bienvenidos al canal.\nHoy vamos a editar un video.\n"
        );
    }

    #[test]
    fn ass_writes_a_style_and_timed_events() {
        let captions = group(&heard(), Grouping::Phrase);
        let ass = write_ass(&captions, &AssLook::default());
        assert!(
            ass.starts_with("[Script Info]\nScriptType: v4.00+\nPlayResX: 1080\nPlayResY: 1920\n")
        );
        assert!(ass.contains(
            "Style: Default,Hanken Grotesk,72,&H00FFFFFF,&H00FFFFFF,&H00000000,&H80000000,-1,0,"
        ));
        assert!(ass.contains(
            "Dialogue: 0,0:00:00.00,0:00:02.00,Default,,0,0,0,,Hola a todos, bienvenidos al canal.\n"
        ));
        assert!(ass.contains("Dialogue: 0,0:00:03.00,0:00:04.60,Default,,0,0,0,,Hoy vamos"));
        assert!(!ass.contains("\\k"));
    }

    #[test]
    fn ass_karaoke_tags_hold_each_word_until_the_next() {
        let captions = group(&heard(), Grouping::Phrase);
        let look = AssLook {
            spoken: Some("#ffe600".to_owned()),
            ..AssLook::default()
        };
        let ass = write_ass(&captions, &look);
        // Yellow is the primary (BGR), white the secondary it fills from.
        assert!(ass.contains(",&H0000E6FF,&H00FFFFFF,"));
        assert!(ass.contains(
            ",,{\\k30}Hola {\\k10}a {\\k50}todos, {\\k60}bienvenidos {\\k10}al {\\k40}canal.\n"
        ));
    }

    #[test]
    fn ass_escapes_braces_and_reads_colours() {
        assert_eq!(ass_colour("#ff8000"), "&H000080FF");
        assert_eq!(ass_colour("nonsense"), "&H00FFFFFF");
        let captions = vec![Caption {
            start: 0.0,
            end: 1.0,
            text: "{\\b1}hola".to_owned(),
            words: Vec::new(),
        }];
        let ass = write_ass(&captions, &AssLook::default());
        assert!(ass.contains(",,｛＼b1｝hola\n"));
    }
}
