use futures::future::{BoxFuture, FutureExt};
use logic::{ObjDetails, Team, Unit};
use logic_ext::Direction;
use rand::prelude::SliceRandom;
use rand::Rng;
use std::collections::{BTreeMap, HashMap};

use std::io::Write;

use std::fs::OpenOptions;
use std::time::Instant;

use expression::Expression;
use serde::Serialize;

use crate::expression::Move;

mod expression;
mod logic_ext;

const FIRST_NAMES: [&'static str; 4096] = include!("../first-names.json");
const LAST_NAMES: [&'static str; 4096] = include!("../last-names.json");

const NUM_ROBOTS: usize = 200;
const SURVIVING_ROBOTS: usize = 50;
const NUM_SPECIES: usize = 15;
const CROSSOVER_INTERVAL: usize = 5;
const MIN_BOTS_PER_SPECIES: usize = 3;
const NUMER_OF_GAMES_PER_BOT_PER_ROUND: usize = 3;

fn generate_bot<Rng: rand::Rng>(rng: &mut Rng) -> Bot {
    let mut expression = expression::Expression {
        kind: expression::ExpressionKind::ConstantMove(Move::Attack(Direction::South)),
    };
    for _i in 0..10 {
        expression.mutate(rng);
    }

    return Bot {
        logic: expression.simplify(),
        species: Species(rng.next_u64()),
        wins: (0, 0, 0),
    };
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct Species(u64);

impl std::fmt::Display for Species {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name_index = self.0 % 4096;
        let _last_name_index = self.0 / 4096 % 4096;

        write!(
            f,
            "{} {}",
            FIRST_NAMES[name_index as usize], LAST_NAMES[name_index as usize]
        )
    }
}

#[derive(Debug, Clone)]
struct Bot {
    logic: Expression,
    species: Species,
    wins: (usize, usize, usize),
}

fn cull_bots<RNG: rand::Rng>(
    bots: Vec<Bot>,
    target_species: usize,
    target_bots: usize,
    rng: &mut RNG,
) -> Vec<Bot> {
    let mut species = HashMap::new();
    let mut species_scores = HashMap::new();

    let get_remaining_bots = |species: &HashMap<Species, Vec<Bot>>| {
        species.values().map(|k: &Vec<Bot>| k.len()).sum::<usize>()
    };

    for bot in bots.into_iter() {
        let k = species_scores
            .entry(bot.species)
            .or_insert((usize::MIN, usize::MIN, usize::MIN));
        *k = (*k).max(bot.wins);
        species.entry(bot.species).or_insert(vec![]).push(bot);
    }

    let mut top_species: Vec<_> = species_scores.iter().collect();
    top_species.sort_by_key(|d| d.1);
    top_species.reverse();
    print!("\t Top species:");
    for (specie, score) in top_species.iter().take(5) {
        print!("{specie} ({score:?}), ")
    }
    println!("");

    while species.len() > target_species {
        if let Some(&specie_to_remove) = species.keys().min_by_key(|t| species_scores.get(t)) {
            if get_remaining_bots(&species)
                - species.get(&specie_to_remove).map(|d| d.len()).unwrap_or(0)
                >= target_bots
            {
                species.remove(&specie_to_remove);
            } else {
                break;
            }
        } else {
            break;
        }
    }

    while get_remaining_bots(&species) > target_bots {
        let mut candidates = species
            .values_mut()
            .filter(|d| d.len() >= MIN_BOTS_PER_SPECIES + 1)
            .collect::<Vec<_>>();
        if candidates.len() == 0 {
            break;
        }
        let length = candidates.len();
        if let Some(candiate) = candidates.get_mut(rng.gen_range(0..length)) {
            candiate.swap_remove(
                rng.gen_range(1..candiate.len())
                    .max(rng.gen_range(1..candiate.len())),
            );
        }
    }

    return species
        .into_iter()
        .flat_map(|(_keys, value)| value)
        .collect();
}

fn crossover<RNG: rand::Rng>(bots: &[Bot], rng: &mut RNG) -> Bot {
    let first_species = bots[0].species;
    let next_bot_index = bots.partition_point(|d| d.species == first_species);

    let next_genome = bots[0]
        .logic
        .clone()
        .crossover(bots[next_bot_index].logic.clone(), rng);

    return Bot {
        logic: next_genome,
        species: Species(rng.next_u64()),
        wins: (0, 0, 0),
    };
}

fn run_batch<'a>(bots: &'a mut [Bot], iterations: usize) -> BoxFuture<'a, ()> {
    const MAX_SCORE: usize = 4 * NUMER_OF_GAMES_PER_BOT_PER_ROUND;

    async move {
        if bots.len() == 0 {
            return;
        }

        for bot in bots.iter_mut() {
            bot.wins.0 *= MAX_SCORE;
        }

        for i in 0..bots.len() {
            for offset in 1..NUMER_OF_GAMES_PER_BOT_PER_ROUND {
                let bot_blue_index = i;
                let bot_red_index = (i + offset) % bots.len();

                let bot_red = bots[bot_blue_index].logic.clone();
                let bot_blue = bots[bot_red_index].logic.clone();

                let mut runners = BTreeMap::new();
                runners.insert(logic::Team::Blue, Ok(bot_red));
                runners.insert(logic::Team::Red, Ok(bot_blue));

                let result = logic::run(
                    runners,
                    |_| (),
                    100,
                    true,
                    None,
                    logic::GameMode::Normal,
                    None,
                )
                .await;

                match result.winner {
                    None => {
                        bots[bot_blue_index].wins.0 += 1;
                        bots[bot_red_index].wins.0 += 1;
                    }
                    Some(logic::Team::Blue) => {
                        bots[bot_blue_index].wins.0 += 2;
                    }
                    Some(logic::Team::Red) => {
                        bots[bot_red_index].wins.0 += 2;
                    }
                }

                let (total_red_health, total_blue_health, total_red_units, total_blue_units) =
                    result.turns[result.turns.len() - 1]
                        .state
                        .objs
                        .values()
                        .fold(
                            (0, 0, 0, 0),
                            |(red_health, blue_health, red_units, blue_units), b| match b.1 {
                                ObjDetails::Unit(Unit { team, health, .. }) => match team {
                                    Team::Red => (
                                        red_health + health,
                                        blue_health,
                                        red_units + 1,
                                        blue_units,
                                    ),
                                    Team::Blue => (
                                        red_health,
                                        blue_health + health,
                                        red_units,
                                        blue_units + 1,
                                    ),
                                },
                                _ => (red_health, blue_health, red_units, blue_units),
                            },
                        );

                bots[bot_blue_index].wins.1 += total_blue_units;
                bots[bot_red_index].wins.1 += total_red_units;

                bots[bot_blue_index].wins.2 += total_blue_health;
                bots[bot_red_index].wins.2 += total_red_health;
            }
        }

        bots.sort_by_key(|t| t.wins);

        if iterations > 1 {
            let winners_end = bots.partition_point(|k| k.wins.0 % MAX_SCORE < MAX_SCORE / 2);
            let tiers_end = bots.partition_point(|k| k.wins.0 % MAX_SCORE < MAX_SCORE * 2 / 3);

            run_batch(&mut bots[0..winners_end], iterations - 1).await;
            run_batch(&mut bots[winners_end..tiers_end], iterations - 1).await;
            run_batch(&mut bots[tiers_end..], iterations - 1).await;
        }
    }
    .boxed()
}

#[tokio::main]
async fn main() {
    // println!("{:?}", std::fs::read_dir(".").unwrap().collect::<Vec<_>>());
    let mut rng = rand::thread_rng();

    let mut bots = (0..NUM_ROBOTS)
        .map(|_| generate_bot(&mut rng))
        .collect::<Vec<_>>();

    for i in 0.. {
        for bot in bots.iter_mut() {
            bot.wins = (0, 0, 0);
        }

        bots.shuffle(&mut rng);

        let global_start_time = Instant::now();

        run_batch(&mut bots[..], 4).await;

        println!("\tWins: {:?}", bots[bots.len() - 1].wins);
        println!("\tWins: {:}", bots[bots.len() - 1].species);
        println!("\tWins: {}", bots[bots.len() - 1].logic);

        let global_end_time = Instant::now();

        bots.reverse();

        let mut species = HashMap::new();
        for bot in bots.iter() {
            let length = species.len();

            species.entry(bot.species).or_insert(
                [
                    '-', '#', '*', '&', '$', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
                ]
                .get(length)
                .unwrap_or(&'_'),
            );
        }

        print!("\t[");
        for i in 0..100 {
            print!("{}", species.get(&bots[i].species).unwrap_or(&&'_'))
        }
        println!("]");
        println!(
            "Iteration {i} took {:?}",
            global_end_time - global_start_time
        );

        let culled_length = SURVIVING_ROBOTS;
        if i % CROSSOVER_INTERVAL == CROSSOVER_INTERVAL - 1 {
            bots = cull_bots(bots, NUM_SPECIES - 1, culled_length, &mut rng);
            bots.push(crossover(&bots, &mut rng));
        } else {
            bots = cull_bots(bots, NUM_SPECIES, culled_length, &mut rng);
        }

        while bots.len() < NUM_ROBOTS {
            let mut bot_copy = bots[rng
                .gen_range(0..culled_length)
                .min(rng.gen_range(0..culled_length))]
            .clone();
            bot_copy.logic.mutate(&mut rng);
            bot_copy.logic = bot_copy.logic.simplify().simplify().simplify();
            bots.push(bot_copy);
        }

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(format!("bots_tmp/{i}.py"))
            .unwrap();
        write!(file, "{}", bots[0].logic).unwrap();
    }

    // println!("{:?}", std::fs::read_dir("..").unwrap().collect::<Vec<_>>());
    // println!(
    //     "{:?}",
    //     std::fs::read_dir("../robotrumble-windows")
    //         .unwrap()
    //         .collect::<Vec<_>>()
    // );
}

#[derive(Serialize)]
struct GameSpec {
    red: String,
    blue: String,
    seed: Option<String>,
}
