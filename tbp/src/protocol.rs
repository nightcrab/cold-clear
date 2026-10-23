//! TBP2: `back_to_back` is the B2B chain
//! length rather than a bool, `start` carries `meter` and `opponents`, and `rules` carries the
//! settings a bot advertised a feature for (`threads` for `set_threads`, `min_nodes` for
//! `set_min_nodes`). Plain TBP1 frontends still work: a bool `back_to_back` is accepted, and fields
//! and message types we don't know are ignored, as the spec asks.

use serde::{Deserialize, Serialize};
use serde_big_array::big_array;

big_array!(BigArray; 40);

/// The features advertised in `info`; the frontend sends the matching `rules` fields only to bots
/// that advertise them.
pub const FEATURES: &[&str] = &["set_threads", "set_min_nodes"];

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum FrontendMessage {
    Rules {
        #[serde(default)]
        randomizer: RandomizerRule,
        /// search threads (`set_threads`)
        #[serde(default)]
        threads: Option<u32>,
        /// nodes to search before suggesting (`set_min_nodes`)
        #[serde(default)]
        min_nodes: Option<u32>,
    },
    Start {
        hold: Option<Piece>,
        queue: Vec<Piece>,
        combo: u32,
        /// the B2B chain length; a TBP1 bool reads as 0 or 1
        #[serde(deserialize_with = "b2b_from_bool_or_int")]
        back_to_back: u32,
        #[serde(with = "BigArray")]
        board: [[Option<char>; 10]; 40],
        #[serde(default)]
        randomizer: RandomizerState,
    },
    Stop,
    Suggest,
    Play {
        #[serde(rename = "move")]
        mv: Move,
    },
    NewPiece {
        piece: Piece,
    },
    Quit,
    #[serde(other)]
    Unknown,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum BotMessage {
    Error {
        reason: ErrorCause,
    },
    Ready,
    Info {
        name: String,
        version: String,
        author: String,
        features: Vec<String>,
    },
    Suggestion {
        moves: Vec<Move>,
    },
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Piece {
    I,
    O,
    T,
    L,
    J,
    S,
    Z,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub location: PieceLocation,
    pub spin: Spin,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PieceLocation {
    #[serde(rename = "type")]
    pub kind: Piece,
    pub orientation: Orientation,
    pub x: i32,
    pub y: i32,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    North,
    South,
    East,
    West,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Spin {
    None,
    Mini,
    Full,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCause {
    UnsupportedRules,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RandomizerRule {
    Uniform,
    SevenBag,
    #[serde(other)]
    Unknown,
}

impl Default for RandomizerRule {
    fn default() -> Self {
        RandomizerRule::Unknown
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum RandomizerState {
    Uniform,
    /// the pieces left in the current bag after the last piece of the queue
    SevenBag { bag_state: Vec<Piece> },
    #[serde(other)]
    Unknown,
}

impl Default for RandomizerState {
    fn default() -> Self {
        RandomizerState::Unknown
    }
}

fn b2b_from_bool_or_int<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum B2b {
        Flag(bool),
        Chain(u32),
    }
    Ok(match B2b::deserialize(d)? {
        B2b::Flag(b) => b as u32,
        B2b::Chain(n) => n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_board() -> String {
        let row = format!("[{}]", vec!["null"; 10].join(","));
        format!("[{}]", vec![row; 40].join(","))
    }

    #[test]
    fn stadium_start() {
        let board = empty_board();
        let msg = format!(
            r#"{{"back_to_back":3,"board":{b},"combo":1,"hold":"L","meter":4,
                "opponents":[{{"back_to_back":0,"board":{b},"combo":0,"hold":null,"meter":0,"queue":["I"]}}],
                "queue":["J","O","T","Z","I","L"],
                "randomizer":{{"type":"seven_bag","bag_state":["S"]}},"type":"start"}}"#,
            b = board
        );
        match serde_json::from_str(&msg).unwrap() {
            FrontendMessage::Start {
                back_to_back,
                combo,
                hold,
                queue,
                randomizer,
                ..
            } => {
                assert_eq!(back_to_back, 3);
                assert_eq!(combo, 1);
                assert_eq!(hold, Some(Piece::L));
                assert_eq!(queue.len(), 6);
                assert_eq!(
                    randomizer,
                    RandomizerState::SevenBag {
                        bag_state: vec![Piece::S]
                    }
                );
            }
            other => panic!("parsed as {:?}", other),
        }
    }

    #[test]
    fn tbp1_start() {
        let msg = format!(
            r#"{{"type":"start","hold":null,"queue":["T"],"combo":0,"back_to_back":true,"board":{}}}"#,
            empty_board()
        );
        match serde_json::from_str(&msg).unwrap() {
            FrontendMessage::Start {
                back_to_back,
                randomizer,
                ..
            } => {
                assert_eq!(back_to_back, 1);
                assert_eq!(randomizer, RandomizerState::Unknown);
            }
            other => panic!("parsed as {:?}", other),
        }
    }

    #[test]
    fn rules_settings() {
        let msg = r#"{"type":"rules","threads":8,"min_nodes":2000}"#;
        assert_eq!(
            serde_json::from_str::<FrontendMessage>(msg).unwrap(),
            FrontendMessage::Rules {
                randomizer: RandomizerRule::Unknown,
                threads: Some(8),
                min_nodes: Some(2000),
            }
        );
        let msg = r#"{"type":"rules"}"#;
        assert_eq!(
            serde_json::from_str::<FrontendMessage>(msg).unwrap(),
            FrontendMessage::Rules {
                randomizer: RandomizerRule::Unknown,
                threads: None,
                min_nodes: None,
            }
        );
    }

    #[test]
    fn unknown_message() {
        let msg = r#"{"type":"something_new","x":1}"#;
        assert_eq!(
            serde_json::from_str::<FrontendMessage>(msg).unwrap(),
            FrontendMessage::Unknown
        );
    }

    #[test]
    fn play_with_opponents() {
        let msg = r#"{"move":{"location":{"orientation":"east","type":"I","x":9,"y":8},"spin":"none"},"opponents":[],"type":"play"}"#;
        match serde_json::from_str(msg).unwrap() {
            FrontendMessage::Play { mv } => {
                assert_eq!(mv.location.kind, Piece::I);
                assert_eq!(mv.location.orientation, Orientation::East);
            }
            other => panic!("parsed as {:?}", other),
        }
    }
}
