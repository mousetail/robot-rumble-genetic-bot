use futures::future::{BoxFuture, FutureExt};
use logic_ext::Direction;
use rand::Rng;
use std::collections::BTreeMap;
use std::io::BufRead;
use std::io::{BufReader, Write};
use std::process::{Command, ExitStatus};
use std::time::Instant;
use std::{fs::OpenOptions, path::PathBuf};

use expression::Expression;
use serde::Serialize;

use crate::expression::{Move};

mod expression;
mod logic_ext;

fn generate_bot<Rng: rand::Rng>(rng: &mut Rng) -> Bot {
    let mut expression = expression::Expression {
        kind: expression::ExpressionKind::ConstantMove(Move::Attack(Direction::South)),
    };
    for i in 0..10 {
        expression.mutate(rng);
    }

    return Bot {
        logic: expression.simplify(),
        species: Species(rng.next_u64() as usize),
        wins: 0,
    };
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Species(usize);

#[derive(Debug, Clone)]
struct Bot {
    logic: Expression,
    species: Species,
    wins: usize,
}

fn run_batch<'a>(bots: &'a mut [Bot], iterations: usize) -> BoxFuture<'a, ()> {
    async move {
        if bots.len() == 0 {
            return;
        }

        for bot in bots.iter_mut() {
            bot.wins *= 3;
        }

        for i in 0..bots.len() {
            let bot_a_index = i;
            let bot_b_index = (i + 1) % bots.len();

            let bot_a = bots[bot_a_index].logic.clone();
            let bot_b = bots[bot_b_index].logic.clone();

            let mut runners = BTreeMap::new();
            runners.insert(logic::Team::Blue, Ok(bot_a));
            runners.insert(logic::Team::Red, Ok(bot_b));

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
                    // bots[bot_a_index].wins += 1;
                    // bots[bot_b_index].wins += 1
                }
                Some(logic::Team::Blue) => {
                    bots[bot_a_index].wins += 1;
                }
                Some(logic::Team::Red) => {
                    bots[bot_b_index].wins += 1;
                }
            }
        }

        bots.sort_by_key(|t| t.wins);

        println!("\tbest bot wins: {}", bots[bots.len() - 1].wins);

        if iterations > 1 {
            let winners_end = bots.partition_point(|k| k.wins % 3 == 0);
            let tiers_end = bots.partition_point(|k| k.wins % 3 != 2);

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

    let mut bots = (0..100).map(|_| generate_bot(&mut rng)).collect::<Vec<_>>();

    for i in 0..25 {
        for bot in bots.iter_mut() {
            bot.wins = 0;
        }

        let global_start_time = Instant::now();

        run_batch(&mut bots[..], 4).await;

        println!("\tWins: {:?}", bots[bots.len() - 1].wins);
        println!("\tWins: {:?}", bots[bots.len() - 1].species);
        println!("\tWins: {}", bots[bots.len() - 1].logic);

        let global_end_time = Instant::now();
        println!(
            "Iteration {i} took {:?}",
            global_end_time - global_start_time
        );

        bots.reverse();
        bots.truncate(bots.len() / 4);

        while bots.len() < 100 {
            let mut bot_copy = bots[rng.gen_range(0..bots.len())].clone();
            bot_copy.wins = 0;
            for _ in 0..25 {
                bot_copy.logic.mutate(&mut rng)
            }
            bot_copy.logic = bot_copy.logic.simplify().simplify().simplify();
            bots.push(bot_copy);
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
