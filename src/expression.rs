use std::path::Display;

use logic::RobotRunner;
use rand::seq::SliceRandom;

#[derive(Debug, Clone)]
pub struct Expression {
    pub kind: ExpressionKind,
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

impl Expression {
    pub fn mutate<RAND: rand::Rng>(&mut self, rng: &mut RAND) {
        self.kind.mutate(rng)
    }
}
#[derive(Debug, Clone)]
pub enum ExpressionKind {
    If {
        condition: Box<Expression>,
        then: Box<Expression>,
        otherwise: Box<Expression>,
    },
    ConstantNumber(i32),
    ConstantBoolean(bool),
    ConstantMove(Move),
    Health,
    X,
    Y,
    GreaterThan {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Equals {
        left: Box<Expression>,
        right: Box<Expression>,
    },
}
#[derive(Debug)]
pub enum ValueType {
    Number,
    Boolean,
    Move,
}

impl ExpressionKind {
    pub fn get_type(&self) -> ValueType {
        match self {
            ExpressionKind::If {
                condition,
                then,
                otherwise,
            } => then.kind.get_type(),
            ExpressionKind::ConstantNumber(_) => ValueType::Number,
            ExpressionKind::ConstantBoolean(_) => ValueType::Boolean,
            ExpressionKind::ConstantMove(_) => ValueType::Move,
            ExpressionKind::Health => ValueType::Number,
            ExpressionKind::X => ValueType::Number,
            ExpressionKind::Y => ValueType::Number,
            ExpressionKind::GreaterThan { left, right } => ValueType::Boolean,
            ExpressionKind::Equals { left, right } => ValueType::Boolean,
        }
    }

    fn generate_move_expression<RAND: rand::Rng>(rng: &mut RAND) -> ExpressionKind {
        let direction = [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ]
        .choose(rng)
        .unwrap();

        return ExpressionKind::ConstantMove(Move::Attack(*direction));
    }

    fn generate_integer_expression<RAND: rand::Rng>(rng: &mut RAND) -> ExpressionKind {
        return [
            ExpressionKind::Health,
            ExpressionKind::X,
            ExpressionKind::Y,
            ExpressionKind::ConstantNumber(*[0, 1, -1, 5, -5].choose(rng).unwrap()),
        ]
        .choose(rng)
        .unwrap()
        .clone();
    }

    fn generate_boolean_expression<RAND: rand::Rng>(rng: &mut RAND) -> ExpressionKind {
        let left = Box::new(Expression {
            kind: Self::generate_integer_expression(rng),
        });
        let right = Box::new(Expression {
            kind: Self::generate_integer_expression(rng),
        });

        if rng.gen_bool(0.5) {
            return ExpressionKind::Equals { left, right };
        } else {
            return ExpressionKind::GreaterThan { left, right };
        }
    }

    pub fn mutate<RAND: rand::Rng>(&mut self, rng: &mut RAND) {
        if rng.gen_bool(0.2) {
            let right = match self.get_type() {
                ValueType::Boolean => Self::generate_boolean_expression(rng),
                ValueType::Number => Self::generate_integer_expression(rng),
                ValueType::Move => Self::generate_move_expression(rng),
            };

            let condition = Self::generate_boolean_expression(rng);

            *self = ExpressionKind::If {
                then: Box::new(Expression { kind: self.clone() }),
                otherwise: Box::new(Expression { kind: right }),
                condition: Box::new(Expression { kind: condition }),
            };
            return;
        }
        match self {
            ExpressionKind::ConstantNumber(t) => {
                *t += rng.sample(rand::distributions::Uniform::new(-1, 2));
            }
            ExpressionKind::If {
                condition,
                then,
                otherwise,
            } => {
                if rng.gen_bool(0.5) {
                    then.mutate(rng);
                } else {
                    otherwise.mutate(rng);
                }
            }
            ExpressionKind::ConstantBoolean(b) => *b = !*b,
            ExpressionKind::ConstantMove(_) => *self = Self::generate_move_expression(rng),
            ExpressionKind::Health => (),
            ExpressionKind::X => (),
            ExpressionKind::Y => (),
            ExpressionKind::GreaterThan { left, right } => {
                if rng.gen_bool(0.5) {
                    left.mutate(rng)
                } else {
                    right.mutate(rng)
                }
            }
            ExpressionKind::Equals { left, right } => {
                if rng.gen_bool(0.5) {
                    left.mutate(rng)
                } else {
                    right.mutate(rng)
                }
            }
        }
    }
}

impl core::fmt::Display for ExpressionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpressionKind::If {
                condition,
                then,
                otherwise,
            } => write!(f, "({then}) if ({condition}) else ({otherwise})"),
            ExpressionKind::ConstantNumber(num) => write!(f, "{num}"),
            ExpressionKind::ConstantBoolean(b) => {
                write!(f, "{}", if *b { "True" } else { "False" })
            }
            ExpressionKind::ConstantMove(mv) => write!(f, "{mv}"),
            ExpressionKind::Health => write!(f, "unit.health"),
            ExpressionKind::X => write!(f, "unit.coords.x"),
            ExpressionKind::Y => write!(f, "unit.coords.y"),
            ExpressionKind::GreaterThan { left, right } => write!(f, "({left}) >= ({right})"),
            ExpressionKind::Equals { left, right } => write!(f, "({left}) == ({right})"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Move {
    Attack(Direction),
    Move(Direction),
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Move::Attack(v) => write!(f, "Action.attack({v})"),
            Move::Move(v) => write!(f, "Action.move({v})"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Direction.{}",
            match self {
                Direction::North => "North",
                Direction::East => "East",
                Direction::South => "South",
                Direction::West => "West",
            }
        )
    }
}

#[async_trait::async_trait]
impl RobotRunner for Expression {
    async fn run(&mut self, input: logic::ProgramInput<'_>) -> logic::ProgramResult {
        Err(logic::ProgramError::NoData)
    }
}