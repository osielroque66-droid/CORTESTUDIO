// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer Abdullah and Concat contributors

//! A title's words in time: when each is said, and how the one being said
//! is told from the rest.
//!
//! A caption made from a voice knows more than its words: it knows when
//! each was spoken. [`Karaoke`] keeps that on the title - one [`WordTime`]
//! per word, in the words' order - with the look the spoken word takes
//! ([`RunStyle`], the same overrides a run of the title has) and how the
//! look moves through the words ([`KaraokeMode`]).
//!
//! The times belong to words by their order, not by their characters: the
//! third time is the third word's, whatever it is spelled as. Correcting a
//! word a transcriber misheard therefore changes nothing here, which is
//! the edit a caption gets most. Adding or removing a word leaves a count
//! that no longer matches, and [`Karaoke::fitted`] then lays the same span
//! of time over the words there are, each by its length.
//!
//! The times are seconds on the clip's source clock, as footage's are:
//! zero is where the clip began when it was made, and trimming its head
//! or splitting it moves `Clip::source_start` and not these. The halves of
//! a split caption each light their own words at the moment they always
//! did.
//!
//! Nothing here paints. [`TextStyle::karaoke_states`] says which stretches
//! of the clip look alike and what runs each wears over the title's own;
//! the title painter paints one picture per state, and the compositor
//! shows each for its stretch.

use serde::{Deserialize, Serialize};

use crate::model::{RunStyle, TextRun, TextStyle};

/// When one word is said, in seconds on the clip's source clock.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(description = "When a word is said."))]
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WordTime {
    /// Where the word begins.
    #[cfg_attr(
        feature = "schema",
        schemars(description = "Seconds from clip origin.")
    )]
    pub at: f64,
    /// Where it ends; never before `at`.
    #[cfg_attr(feature = "schema", schemars(description = "Seconds."))]
    pub until: f64,
}

/// How the spoken look moves through the words.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(description = "How the look moves."))]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KaraokeMode {
    /// The word being said wears the look; the rest are the title's.
    #[default]
    Active,
    /// Every word said so far wears it, and keeps it: the line fills in
    /// as it is spoken, the way a karaoke screen does.
    Fill,
}

/// A title's words in time. See the module docs.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(description = "A title's words in time."))]
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Karaoke {
    /// One time per word of the title's content, in the words' order.
    #[cfg_attr(
        feature = "schema",
        schemars(description = "One per word of content, in order.")
    )]
    pub words: Vec<WordTime>,
    /// How the look moves through the words.
    #[cfg_attr(
        feature = "schema",
        schemars(description = "active: spoken word; fill: all said so far.")
    )]
    pub mode: KaraokeMode,
    /// The look of a spoken word, over the title's own and over any run
    /// the word already has. Empty changes nothing on screen: the times
    /// are then only kept, for an export of subtitles or a look given
    /// later.
    // The same overrides a run has; named, not spelled out again, in a
    // schema that sits in an agent's context once per tool that takes a
    // title's style.
    #[cfg_attr(
        feature = "schema",
        schemars(
            with = "serde_json::Value",
            description = "The spoken word look: a run's fields (color, fontWeight, fontScale...)."
        )
    )]
    pub style: RunStyle,
}

/// A pause between two words longer than this lets the look go until the
/// next word begins; a shorter one is bridged, so a line spoken at an
/// ordinary pace never flickers between its words.
const BRIDGE: f64 = 0.45;

/// The character ranges of `content`'s words: what whitespace separates,
/// punctuation and all, so "¿Cómo" and "estás?" are a word each.
pub fn word_ranges(content: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut begun: Option<usize> = None;
    let mut count = 0;
    for (index, ch) in content.chars().enumerate() {
        count = index + 1;
        match (ch.is_whitespace(), begun) {
            (true, Some(start)) => {
                ranges.push((start, index));
                begun = None;
            }
            (false, None) => begun = Some(index),
            _ => {}
        }
    }
    if let Some(start) = begun {
        ranges.push((start, count));
    }
    ranges
}

/// `count` words' times over `from..until`, each word's share of the span
/// its share of the characters: what a caption with no measured times is
/// given, and what a line whose words were recounted falls back to.
/// `lengths` are the words' lengths in characters.
pub fn spread(lengths: &[usize], from: f64, until: f64) -> Vec<WordTime> {
    let total: usize = lengths.iter().map(|length| (*length).max(1)).sum();
    let span = (until - from).max(0.0);
    let mut at = from;
    lengths
        .iter()
        .map(|length| {
            let share = span * (*length).max(1) as f64 / total.max(1) as f64;
            let word = WordTime {
                at,
                until: at + share,
            };
            at += share;
            word
        })
        .collect()
}

impl Karaoke {
    /// Every number made one a caption can be drawn from: finite, no word
    /// ending before it begins, and none beginning before the one ahead
    /// of it did.
    pub fn tidy(mut self) -> Karaoke {
        let mut floor = f64::NEG_INFINITY;
        for word in &mut self.words {
            if !word.at.is_finite() {
                word.at = if floor.is_finite() { floor } else { 0.0 };
            }
            word.at = word.at.max(floor);
            if !word.until.is_finite() || word.until < word.at {
                word.until = word.at;
            }
            floor = word.at;
        }
        self.style = self.style.tidy();
        self
    }

    /// The times of `content`'s words: the ones kept when there is one for
    /// each word, and otherwise the same span of time spread over the
    /// words there are now, by their lengths. Empty when nothing was kept.
    pub fn fitted(&self, content: &str) -> Vec<WordTime> {
        let ranges = word_ranges(content);
        if ranges.len() == self.words.len() {
            return self.words.clone();
        }
        let (Some(first), Some(last)) = (self.words.first(), self.words.last()) else {
            return Vec::new();
        };
        let lengths: Vec<usize> = ranges.iter().map(|(start, end)| end - start).collect();
        spread(&lengths, first.at, last.until.max(first.at))
    }

    /// The times moved by `seconds`: what a caption cut from a longer one
    /// carries, its clock starting where its own first frame is.
    pub fn shifted(mut self, seconds: f64) -> Karaoke {
        for word in &mut self.words {
            word.at += seconds;
            word.until += seconds;
        }
        self
    }
}

/// One stretch of a karaoke title that looks the same throughout.
#[derive(Clone, PartialEq, Debug)]
pub struct KaraokeState {
    /// Seconds into the clip where the stretch begins.
    pub from: f64,
    /// Seconds into the clip where it ends.
    pub until: f64,
    /// The title as it is drawn over the stretch: its own style with the
    /// spoken words' look laid over their characters, and no karaoke of
    /// its own.
    pub style: TextStyle,
}

impl TextStyle {
    /// The stretches of a clip `duration` seconds long, begun
    /// `source_start` seconds into its source clock, over which this
    /// title looks the same, in order and with no gap between them: the
    /// whole clip is covered. None for a title that is one picture
    /// throughout - no karaoke, no look for the spoken word, or no word
    /// said while the clip is up - which is then painted as it always was.
    pub fn karaoke_states(&self, source_start: f64, duration: f64) -> Option<Vec<KaraokeState>> {
        let karaoke = self.karaoke.as_ref()?;
        if karaoke.style.is_empty() || !duration.is_finite() || duration <= 0.0 {
            return None;
        }
        let ranges = word_ranges(&self.content);
        let times = karaoke.fitted(&self.content);
        if times.is_empty() || times.len() != ranges.len() {
            return None;
        }

        // Each word's stretch on the clip's own clock: from where it is
        // said to where the next is, a short pause bridged and a long one
        // left bare.
        let lit: Vec<(f64, f64)> = times
            .iter()
            .enumerate()
            .map(|(index, word)| {
                let until = match times.get(index + 1) {
                    Some(next) if next.at - word.until <= BRIDGE => next.at,
                    _ => word.until,
                };
                (word.at - source_start, until.max(word.at) - source_start)
            })
            .collect();

        // The instants where the picture changes, inside the clip.
        let mut cuts = vec![0.0, duration];
        for &(from, until) in &lit {
            for edge in [from, until] {
                if edge > 0.0 && edge < duration {
                    cuts.push(edge);
                }
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

        let plain = TextStyle {
            karaoke: None,
            ..self.clone()
        };
        let mut states: Vec<(f64, f64, Vec<usize>)> = Vec::new();
        for pair in cuts.windows(2) {
            let (from, until) = (pair[0], pair[1]);
            let middle = (from + until) / 2.0;
            let spoken: Vec<usize> = match karaoke.mode {
                KaraokeMode::Active => lit
                    .iter()
                    .position(|&(begin, end)| middle >= begin && middle < end)
                    .into_iter()
                    .collect(),
                KaraokeMode::Fill => lit
                    .iter()
                    .enumerate()
                    .filter(|&(_, &(begin, _))| middle >= begin)
                    .map(|(index, _)| index)
                    .collect(),
            };
            match states.last_mut() {
                Some(last) if last.2 == spoken => last.1 = until,
                _ => states.push((from, until, spoken)),
            }
        }
        // One picture throughout is no karaoke as far as a painter cares.
        if states.len() == 1 && states[0].2.is_empty() {
            return None;
        }
        Some(
            states
                .into_iter()
                .map(|(from, until, spoken)| {
                    let mut style = plain.clone();
                    for index in spoken {
                        let (start, end) = ranges[index];
                        style.style_range(start, end, &karaoke.style);
                    }
                    KaraokeState { from, until, style }
                })
                .collect(),
        )
    }

    /// Keeps the karaoke fit to be read back: tidied, and gone when it
    /// times no word at all.
    pub fn tidy_karaoke(&mut self) {
        self.karaoke = self
            .karaoke
            .take()
            .map(Karaoke::tidy)
            .filter(|karaoke| !karaoke.words.is_empty());
    }
}

/// The runs a state wears beyond the title's own; for tests and for a
/// caller that only wants to know which words are lit.
pub fn lit_runs(state: &KaraokeState, base: &TextStyle) -> Vec<TextRun> {
    state
        .style
        .runs
        .iter()
        .filter(|run| !base.runs.contains(run))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yellow() -> RunStyle {
        RunStyle {
            color: Some("#ffe600".to_owned()),
            ..RunStyle::default()
        }
    }

    fn title(content: &str, words: &[(f64, f64)], mode: KaraokeMode) -> TextStyle {
        TextStyle {
            content: content.to_owned(),
            karaoke: Some(Karaoke {
                words: words
                    .iter()
                    .map(|&(at, until)| WordTime { at, until })
                    .collect(),
                mode,
                style: yellow(),
            }),
            ..TextStyle::default()
        }
    }

    #[test]
    fn words_are_what_whitespace_separates_in_characters() {
        assert_eq!(
            word_ranges("¿Cómo  estás?\nBien"),
            vec![(0, 5), (7, 13), (14, 18)]
        );
        assert!(word_ranges("   ").is_empty());
    }

    #[test]
    fn the_active_word_is_lit_from_where_it_is_said_to_the_next() {
        let style = title(
            "Hola a todos",
            &[(0.2, 0.5), (0.6, 0.7), (0.8, 1.4)],
            KaraokeMode::Active,
        );
        let states = style.karaoke_states(0.0, 2.0).expect("a karaoke title");
        let spans: Vec<(f64, f64, Vec<(usize, usize)>)> = states
            .iter()
            .map(|state| {
                (
                    state.from,
                    state.until,
                    state
                        .style
                        .runs
                        .iter()
                        .map(|run| (run.start, run.end))
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            spans,
            vec![
                (0.0, 0.2, vec![]),
                (0.2, 0.6, vec![(0, 4)]),
                (0.6, 0.8, vec![(5, 6)]),
                (0.8, 1.4, vec![(7, 12)]),
                (1.4, 2.0, vec![]),
            ]
        );
        // What is painted has no karaoke of its own to recurse into.
        assert!(states.iter().all(|state| state.style.karaoke.is_none()));
        assert_eq!(
            states[1].style.runs[0].style.color.as_deref(),
            Some("#ffe600")
        );
    }

    #[test]
    fn the_states_cover_the_clip_with_no_gap() {
        let style = title(
            "uno dos tres",
            &[(0.1, 0.3), (1.5, 1.8), (1.9, 2.5)],
            KaraokeMode::Active,
        );
        let states = style.karaoke_states(0.0, 3.0).expect("a karaoke title");
        assert_eq!(states.first().map(|state| state.from), Some(0.0));
        assert_eq!(states.last().map(|state| state.until), Some(3.0));
        for pair in states.windows(2) {
            assert_eq!(pair[0].until, pair[1].from);
        }
        // The long pause after "uno" is bare; the short one after "dos"
        // is bridged.
        assert_eq!((states[1].from, states[1].until), (0.1, 0.3));
        assert!(states[2].style.runs.is_empty());
        assert_eq!((states[3].from, states[3].until), (1.5, 1.9));
    }

    #[test]
    fn fill_keeps_every_word_said_so_far() {
        let style = title("uno dos", &[(0.0, 0.4), (0.5, 0.9)], KaraokeMode::Fill);
        let states = style.karaoke_states(0.0, 1.0).expect("a karaoke title");
        assert_eq!(states.len(), 2);
        // Neighbours in one look are one run, as every title's are.
        assert_eq!(states[0].style.runs.len(), 1);
        assert_eq!(
            (states[0].style.runs[0].start, states[0].style.runs[0].end),
            (0, 3)
        );
        let covered: usize = states[1]
            .style
            .runs
            .iter()
            .map(|run| run.end - run.start)
            .sum();
        assert!(
            covered >= 6,
            "both words are lit: {:?}",
            states[1].style.runs
        );
    }

    #[test]
    fn a_trimmed_or_split_clip_lights_its_words_when_it_always_did() {
        let style = title("uno dos", &[(0.0, 0.4), (1.0, 1.4)], KaraokeMode::Active);
        // The tail of a split at 0.8 s: its clock begins 0.8 s in.
        let states = style.karaoke_states(0.8, 1.2).expect("a karaoke title");
        assert_eq!(
            states
                .iter()
                .map(|state| (
                    (state.from * 100.0).round() / 100.0,
                    (state.until * 100.0).round() / 100.0,
                    state.style.runs.len()
                ))
                .collect::<Vec<_>>(),
            vec![(0.0, 0.2, 0), (0.2, 0.6, 1), (0.6, 1.2, 0)]
        );
    }

    #[test]
    fn a_corrected_word_keeps_its_time_and_a_recount_spreads_the_span() {
        let mut style = title("ola mundo", &[(0.0, 0.5), (0.5, 1.5)], KaraokeMode::Active);
        style.content = "Hola mundo".to_owned();
        let karaoke = style.karaoke.clone().expect("karaoke");
        assert_eq!(karaoke.fitted(&style.content), karaoke.words);

        style.content = "Hola a todo el mundo".to_owned();
        let fitted = karaoke.fitted(&style.content);
        assert_eq!(fitted.len(), 5);
        assert_eq!(fitted[0].at, 0.0);
        assert!((fitted[4].until - 1.5).abs() < 1e-9);
        for pair in fitted.windows(2) {
            assert!((pair[0].until - pair[1].at).abs() < 1e-9);
        }
        assert!(style.karaoke_states(0.0, 1.5).is_some());
    }

    #[test]
    fn no_look_or_no_words_is_one_picture() {
        let mut style = title("uno dos", &[(0.0, 0.4), (0.5, 0.9)], KaraokeMode::Active);
        style.karaoke.as_mut().expect("karaoke").style = RunStyle::default();
        assert!(style.karaoke_states(0.0, 1.0).is_none());

        let plain = TextStyle::default();
        assert!(plain.karaoke_states(0.0, 1.0).is_none());

        // Words said only after the clip has gone.
        let late = title("uno dos", &[(5.0, 5.4), (5.5, 5.9)], KaraokeMode::Active);
        assert!(late.karaoke_states(0.0, 1.0).is_none());
    }

    #[test]
    fn the_spoken_look_goes_over_a_run_the_word_already_has() {
        let mut style = title("uno dos", &[(0.0, 0.4), (0.5, 0.9)], KaraokeMode::Active);
        style.style_range(
            0,
            3,
            &RunStyle {
                color: Some("#ff0000".to_owned()),
                font_weight: Some(900.0),
                ..RunStyle::default()
            },
        );
        let states = style.karaoke_states(0.0, 1.0).expect("a karaoke title");
        let first = &states[0].style.runs[0];
        assert_eq!(first.style.color.as_deref(), Some("#ffe600"));
        assert_eq!(first.style.font_weight, Some(900.0), "its weight is kept");
        // Once "dos" is the word, "uno" is back in its own red.
        assert_eq!(
            states[1].style.runs[0].style.color.as_deref(),
            Some("#ff0000")
        );
    }

    #[test]
    fn tidying_orders_the_words_and_drops_an_empty_karaoke() {
        let karaoke = Karaoke {
            words: vec![
                WordTime {
                    at: 1.0,
                    until: 0.5,
                },
                WordTime {
                    at: 0.2,
                    until: f64::NAN,
                },
            ],
            ..Karaoke::default()
        }
        .tidy();
        assert_eq!(
            karaoke.words[0],
            WordTime {
                at: 1.0,
                until: 1.0
            }
        );
        assert_eq!(
            karaoke.words[1],
            WordTime {
                at: 1.0,
                until: 1.0
            }
        );

        let mut style = TextStyle {
            karaoke: Some(Karaoke::default()),
            ..TextStyle::default()
        };
        style.tidy_karaoke();
        assert!(style.karaoke.is_none());
    }

    #[test]
    fn a_document_without_karaoke_is_the_document_it_always_was() {
        let plain = serde_json::to_value(TextStyle::default()).expect("json");
        assert!(plain.get("karaoke").is_none());
        let style = title("uno", &[(0.0, 0.4)], KaraokeMode::Fill);
        let json = serde_json::to_value(&style).expect("json");
        assert_eq!(json["karaoke"]["mode"], "fill");
        assert_eq!(json["karaoke"]["words"][0]["until"], 0.4);
        let back: TextStyle = serde_json::from_value(json).expect("reads back");
        assert_eq!(back, style);
    }
}
