use futures::future::{BoxFuture, FutureExt};
use logic::{MainOutput, ObjDetails, RobotRunner, Team, Unit};
use logic_ext::Direction;
use rand::prelude::SliceRandom;
use rand::Rng;
use sockets::{start_socket, TrainingProgressAnnouncement};
use std::collections::{BTreeMap, HashMap};

use std::io::Write;

use std::fs::OpenOptions;
use std::time::Instant;

use expression::Expression;
use serde::{Serialize};

use crate::expression::Move;

mod expression;
mod logic_ext;
mod sockets;

const FIRST_NAMES: [&'static str; 4096] = include!("../first-names.json");
const LAST_NAMES: [&'static str; 4096] = include!("../last-names.json");

const NUM_ROBOTS: usize = 200;
const SURVIVING_ROBOTS: usize = 50;
const NUM_SPECIES: usize = 15;
const CROSSOVER_INTERVAL: usize = 5;
const MIN_BOTS_PER_SPECIES: usize = 3;
const NUMER_OF_GAMES_PER_BOT_PER_ROUND: usize = 2;
const NUMER_OF_PLAYOFF_ROUNDS: usize = 3;

fn generate_bot<Rng: rand::Rng>(rng: &mut Rng) -> Bot {
    let mut expression = expression::Expression {
        kind: expression::ExpressionKind::ConstantMove(Move::Attack(Direction::South)),
        times_used: 0,
    };
    for _i in 0..10 {
        expression.mutate(rng, true);
    }

    return Bot {
        logic: expression.simplify(),
        species: Species(rng.next_u64()),
        score: Default::default(),
        generation: 0,
        parents: None,
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

impl Serialize for Species {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

#[derive(Debug, Clone, Serialize, Default, Ord, PartialEq, PartialOrd, Eq, Copy)]
struct BotScore {
    wins: [usize; NUMER_OF_PLAYOFF_ROUNDS],
    friendly_units: isize,
    enemy_units: isize,
    friendly_health: isize,
    enemy_health: isize,
    total_wins: usize,
}

#[derive(Debug, Clone, Serialize)]
struct Bot {
    logic: Expression,
    species: Species,
    score: BotScore,
    generation: usize,
    parents: Option<(Species, Species)>,
}

#[async_trait::async_trait]
impl RobotRunner for &mut Bot {
    async fn run(&mut self, input: logic::ProgramInput<'_>) -> logic::ProgramResult {
        self.logic.run(input).await
    }
}

fn cull_bots<RNG: rand::Rng>(
    bots: Vec<Bot>,
    target_species: usize,
    target_bots: usize,
    rng: &mut RNG,
) -> (Vec<Bot>, Vec<(Species, BotScore)>) {
    let mut species = HashMap::new();
    let mut species_scores = HashMap::new();

    let get_remaining_bots = |species: &HashMap<Species, Vec<Bot>>| {
        species.values().map(|k: &Vec<Bot>| k.len()).sum::<usize>()
    };

    for bot in bots.into_iter() {
        let k: &mut BotScore = species_scores
            .entry(bot.species)
            .or_insert(Default::default());
        *k = (*k).max(bot.score);
        species.entry(bot.species).or_insert(vec![]).push(bot);
    }

    let mut top_species: Vec<_> = species_scores.iter().map(|(&a, &b)| (a, b)).collect();
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

    return (
        species
            .into_iter()
            .flat_map(|(_keys, value)| value)
            .collect(),
        top_species,
    );
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
        score: Default::default(),
        generation: 0,
        parents: Some((first_species, bots[next_bot_index].species)),
    };
}

#[allow(unused)]
fn draw_game(game: &MainOutput) {
    for turn in game.turns.iter() {
        let state = &turn.state;

        let mut out = [[' '; 19]; 19];
        for (_id, obj) in state.objs.iter() {
            out[obj.0.coords.1][obj.0.coords.0] = match obj.1 {
                logic::ObjDetails::Terrain(_) => '#',
                logic::ObjDetails::Unit(Unit {
                    team: Team::Blue, ..
                }) => '.',
                logic::ObjDetails::Unit(Unit {
                    team: Team::Red, ..
                }) => '^',
            };
        }

        for line in out {
            for chr in line {
                print!("{chr}");
            }
            println!();
        }

        println!("{:?}", turn.robot_actions);

        println!();
    }
}

async fn run_game(bots: &mut [Bot], blue_index: usize, red_index: usize) -> MainOutput {
    assert!(blue_index < bots.len());
    assert!(red_index < bots.len());
    assert!(red_index != blue_index);
    let max_index = blue_index.max(red_index);
    let (early_parts, late_parts) = bots.split_at_mut(max_index);

    let (blue_bot, red_bot) = if blue_index > red_index {
        (&mut late_parts[0], &mut early_parts[red_index])
    } else {
        (&mut early_parts[blue_index], &mut late_parts[0])
    };

    let mut runners = BTreeMap::new();
    runners.insert(logic::Team::Blue, Ok(blue_bot));
    runners.insert(logic::Team::Red, Ok(red_bot));

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

    result
}

fn run_batch<'a>(bots: &'a mut [Bot], iteration: usize) -> BoxFuture<'a, ()> {
    const MAX_SCORE: usize = 4 * NUMER_OF_GAMES_PER_BOT_PER_ROUND;

    async move {
        if bots.len() == 0 {
            return;
        }

        for i in 0..bots.len() {
            for offset in 1..=NUMER_OF_GAMES_PER_BOT_PER_ROUND {
                let bot_blue_index = i;
                let bot_red_index = (i + offset) % bots.len();

                if bot_blue_index == bot_red_index {
                    continue;
                }

                let result = run_game(bots, bot_blue_index, bot_red_index).await;

                match result.winner {
                    None => {
                        bots[bot_blue_index].score.wins[iteration] += 1;
                        bots[bot_red_index].score.wins[iteration] += 1;
                    }
                    Some(logic::Team::Blue) => {
                        bots[bot_blue_index].score.wins[iteration] += 2;
                    }
                    Some(logic::Team::Red) => {
                        bots[bot_red_index].score.wins[iteration] += 2;
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
                                        red_health + health as isize,
                                        blue_health,
                                        red_units + 1,
                                        blue_units,
                                    ),
                                    Team::Blue => (
                                        red_health,
                                        blue_health + health as isize,
                                        red_units,
                                        blue_units + 1,
                                    ),
                                },
                                _ => (red_health, blue_health, red_units, blue_units),
                            },
                        );
                match result.winner {
                    Some(Team::Red) => assert!(total_red_units > total_blue_units),
                    Some(Team::Blue) => assert!(total_blue_units > total_red_units),
                    None => assert_eq!(total_red_units, total_blue_units),
                }

                bots[bot_blue_index].score.friendly_units += total_blue_units;
                bots[bot_blue_index].score.enemy_units -= total_red_units;
                bots[bot_red_index].score.friendly_units += total_red_units;
                bots[bot_red_index].score.enemy_units -= total_blue_units;

                bots[bot_blue_index].score.friendly_health += total_blue_health;
                bots[bot_blue_index].score.enemy_health -= total_red_health;
                bots[bot_red_index].score.enemy_health += total_red_health;
                bots[bot_blue_index].score.enemy_health -= total_blue_health;
            }
        }

        for bot in bots.iter_mut() {
            bot.score.wins[iteration] /= MAX_SCORE / 5;
        }

        bots.sort_by_key(|t| (t.score.wins[iteration], t.score));

        if iteration < NUMER_OF_PLAYOFF_ROUNDS - 1 {
            let segment_size = bots.len() / 3;
            assert!(
                segment_size > NUMER_OF_GAMES_PER_BOT_PER_ROUND,
                "segment size is {segment_size}"
            );

            run_batch(&mut bots[0..segment_size], iteration + 1).await;
            run_batch(&mut bots[segment_size..2 * segment_size], iteration + 1).await;
            run_batch(&mut bots[2 * segment_size..], iteration + 1).await;
        }
    }
    .boxed()
}

#[tokio::main]
async fn main() {
    // println!("{:?}", std::fs::read_dir(".").unwrap().collect::<Vec<_>>());
    let mut rng = rand::thread_rng();

    let mut bots = (0..NUM_ROBOTS - 1)
        .map(|_| generate_bot(&mut rng))
        .collect::<Vec<_>>();

    bots.push(Bot {
        species: Species(0),
        logic: Expression {
            times_used: 0,
            kind: expression::ExpressionKind::If {
                condition: Box::new(Expression {
                    times_used: 0,
                    kind: expression::ExpressionKind::GreaterThan {
                        left: Box::new(Expression {
                            times_used: 0,
                            kind: expression::ExpressionKind::X,
                        }),
                        right: Box::new(Expression {
                            times_used: 0,
                            kind: expression::ExpressionKind::ConstantNumber(9),
                        }),
                    },
                }),
                then: Box::new(Expression {
                    kind: expression::ExpressionKind::ConstantMove(Move::Move(Direction::West)),
                    times_used: 0,
                }),
                otherwise: Box::new(Expression {
                    kind: expression::ExpressionKind::ConstantMove(Move::Move(Direction::East)),
                    times_used: 0,
                }),
            },
        },
        score: Default::default(),
        generation: 0,
        parents: None,
    });
    println!("{}", Species(0));

    let (channel, _) = tokio::sync::broadcast::channel::<TrainingProgressAnnouncement>(16);
    tokio::spawn(start_socket(channel.clone()));

    for i in 0.. {
        for bot in bots.iter_mut() {
            bot.score = Default::default();
        }

        bots.shuffle(&mut rng);

        let global_start_time = Instant::now();

        run_batch(&mut bots[..], 0).await;

        println!("\tWins:\t {:?}", bots[bots.len() - 1].score);
        println!(
            "\tSpecies:\t {:} (generation {})",
            bots[bots.len() - 1].species,
            bots[bots.len() - 1].generation
        );
        if let Some((parent1, parent2)) = bots[bots.len() - 1].parents {
            println!("\tParents:\t{parent1}\t{parent2}")
        }
        println!("\tLogic:\t {}", bots[bots.len() - 1].logic);

        let global_end_time = Instant::now();

        bots.reverse();

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(format!("bots_tmp/{i}.py"))
            .unwrap();
        write!(
            file,
            "#{}
            #{}
            {}",
            serde_json::to_string(&bots[0].logic).unwrap(),
            bots[0].species,
            bots[0].logic
        )
        .unwrap();

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
        for i in 0..100.min(NUM_ROBOTS) {
            print!("{}", species.get(&bots[i].species).unwrap_or(&&'_'))
        }
        println!("]");
        println!(
            "Iteration {i} took {:?}",
            global_end_time - global_start_time
        );

        let best_bot = bots[0].clone();

        let culled_length = SURVIVING_ROBOTS;
        let species_scores;
        if i % CROSSOVER_INTERVAL == CROSSOVER_INTERVAL - 1 {
            (bots, species_scores) = cull_bots(bots, NUM_SPECIES - 1, culled_length, &mut rng);
            bots.push(crossover(&bots, &mut rng));
        } else {
            (bots, species_scores) = cull_bots(bots, NUM_SPECIES, culled_length, &mut rng);
        }

        if channel.receiver_count() > 0 {
            channel
                .send(TrainingProgressAnnouncement {
                    best_bot,
                    species: species_scores,
                    iteration_number: i,
                })
                .unwrap();
        }

        while bots.len() < NUM_ROBOTS {
            let mut bot_copy = bots[rng
                .gen_range(0..culled_length)
                .min(rng.gen_range(0..culled_length))]
            .clone();
            bot_copy.generation += 1;
            bot_copy.logic.mutate(&mut rng, false);
            bot_copy.logic = bot_copy.logic.simplify().simplify().simplify();
            bots.push(bot_copy);
        }

        for bot in bots.iter_mut() {
            bot.logic.clear_times_used();
        }
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
