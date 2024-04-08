use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{Bot, BotScore};

const FIRST_NAMES: [&'static str; 4096] = include!("../first-names.json");
const LAST_NAMES: [&'static str; 4096] = include!("../last-names.json");

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Species(pub u64);

impl std::fmt::Display for Species {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name_index = self.0 % 4096;
        let last_name_index = self.0 / 4096 % 4096;

        write!(
            f,
            "{} {}",
            FIRST_NAMES[name_index as usize], LAST_NAMES[last_name_index as usize]
        )
    }
}

impl Serialize for Species {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for Species {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let name = <String>::deserialize(deserializer)?;

        for (index, first_name) in FIRST_NAMES.iter().enumerate() {
            if name.starts_with(first_name) {
                for (index2, last_name) in LAST_NAMES.iter().enumerate() {
                    if name[first_name.len()+1..].starts_with(last_name) {
                        return Ok(Species((index + index2 * FIRST_NAMES.len()) as u64))
                    }
                }
            }
        }

        Err(serde::de::Error::custom("Invalid data"))
    }
}


#[derive(Serialize, Deserialize, Clone, Debug)]
struct SpeciesInfo {
    round_introduced: usize,
    round_extinct:Option<usize>,
    parents: Option<[Species;2]>,
    best_score: BotScore
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FamilyTree(HashMap<Species, SpeciesInfo>);

impl FamilyTree {
    pub fn new() -> FamilyTree {
        FamilyTree(HashMap::new())
    }

    pub fn analize(&mut self, bots: &[Bot], round_number: usize) {
        let mut new_species = HashMap::new();
        for bot in bots {
            let info = new_species.entry(bot.species).or_insert(SpeciesInfo {
                round_introduced: round_number,
                round_extinct: None,
                parents: bot.parents,
                best_score: bot.score
            });

            info.best_score = info.best_score.max(bot.score);
        }

        for (specie, info) in new_species.iter_mut() {
            info.round_introduced = match self.0.get(specie) {
                Some(t) => t.round_introduced,
                None => round_number
            }
        }

        for (specie, info) in self.0.iter_mut() {
            if let None = new_species.get(specie) {
                info.round_extinct = info.round_extinct.or(Some(round_number));
            }
        }

        self.0.extend(new_species);
    }
}