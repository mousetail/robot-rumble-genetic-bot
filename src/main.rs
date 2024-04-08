use std::io::{BufReader, Write};
use std::process::{Command, ExitStatus};
use std::time::Instant;
use std::{fs::OpenOptions, path::PathBuf};use std::io::BufRead;

use serde::Serialize;

use crate::expression::{Direction, Move};

mod expression;

fn generate_bot<Rng: rand::Rng>(rng: &mut Rng) -> String {
    let mut expression = expression::Expression {
        kind: expression::ExpressionKind::ConstantMove(Move::Attack(Direction::South)),
    };
    for i in 0..10 {
        expression.mutate(rng);
    }

    let code = format!("inline:Python;{}",format!(include_str!("./robot_template.py"), expression));

    return code;
}

fn main() {
    // println!("{:?}", std::fs::read_dir(".").unwrap().collect::<Vec<_>>());
    let mut rng = rand::thread_rng();

    let bots = (0..100).map(|_|generate_bot(&mut rng)).collect::<Vec<_>>();

    let mut specs = Vec::with_capacity(100);
    let global_start_time = Instant::now();

    for i in 0..bots.len() {
        let bot_a = bots[i].clone();
        let bot_b = bots[(i+1)%bots.len()].clone();

        specs.push(
            GameSpec {
                red: bot_a,
                blue: bot_b,
                seed: None
            }
        );

        if specs.len() >= 99 {
            let start_time = Instant::now();        
            let mut child = Command::new("../robot-rumble/cli/target/debug/rumblebot")
            .args(["run", "batch"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

            let mut process_stdin = child.stdin.as_ref().unwrap();
            let mut process_stdout = BufReader::new(child.stdout.as_mut().unwrap()).lines();
            for spec in &specs {
                writeln!(process_stdin, "{}", serde_json::to_string(&spec).unwrap()).unwrap();

                let line = match  process_stdout.next() {
                    None => panic!("Line expected"),
                    Some(Err(t)) => panic!("{t:?}"),
                    Some(Ok(line)) => line
                };

            println!("{i}\t{:?}\t{line}", Instant::now() - global_start_time);
                
            }

            let out = child.wait().unwrap();


            let end_time = Instant::now();
            // let output = std::str::from_utf8(&out.stdout).unwrap().replace("\n", "\n    ");
            // if output.contains("tie") {
            //     continue;
            // }

            specs.clear();
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
    seed: Option<String>
}