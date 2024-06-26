use std::collections::BTreeMap;

use logic::{ActionType, Coords, Id, ObjDetails, RobotRunner, Team, Unit};
use rand::seq::SliceRandom;
use serde::Deserialize;
use serde::Serialize;

use crate::logic_ext::CoordsExt;
use crate::logic_ext::Direction;
use crate::logic_ext::TeamExt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Expression {
    pub kind: ExpressionKind,
    #[serde(skip_deserializing)]
    pub times_used: usize,
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

impl Expression {
    pub fn new(kind: ExpressionKind) -> Expression {
        return Expression {
            kind,
            times_used: 0
        }
    }

    pub fn new_box(kind: ExpressionKind) -> Box<Expression> {
        return Box::new(Self::new(kind));
    }

    pub fn mutate<RAND: rand::Rng>(&mut self, rng: &mut RAND, ignore_sanity_checks: bool) {
        self.kind.mutate(rng, self.times_used, ignore_sanity_checks)
    }

    pub fn eval(&mut self, input: &logic::ProgramInput, id: Id, unit: &Unit) -> Result<Value, ()> {
        self.times_used += 1;
        self.kind.eval(input, id, unit)
    }

    pub fn simplify(self) -> Expression {
        Expression::new(self.kind.simplify())
    }

    pub fn crossover<RNG: rand::Rng>(self, other: Self, rng: &mut RNG) -> Expression {
        return Expression::new(ExpressionKind::If {
                condition: Expression::new_box(ExpressionKind::generate_boolean_expression(rng)),
                then: Box::new(self),
                otherwise: Box::new(other),
            });
    }

    pub fn clear_times_used(&mut self) {
        self.times_used = 0;
        self.kind.clear_times_used();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpressionKind {
    If {
        condition: Box<Expression>,
        then: Box<Expression>,
        otherwise: Box<Expression>,
    },
    ConstantNumber(i32),
    ConstantBoolean(bool),
    ConstantMove(Move),
    AlliedSurroundingTiles,
    EnemySurroundingTiles,
    AttackNearestEnemy,
    MoveToNearestEnemy,
    DistanceToNearestEnemy,
    DistanceToNearestAlly,
    DistanceToCenter,
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
    ClosestEnemyHealth,
    ClosestAllyHealth,
}
#[derive(Debug)]
pub enum ValueType {
    Number,
    Boolean,
    Move,
}

pub enum Value {
    Number(i32),
    Boolean(bool),
    Move(Move),
}

impl ExpressionKind {
    pub fn get_type(&self) -> ValueType {
        match self {
            ExpressionKind::If { then, .. } => then.kind.get_type(),
            ExpressionKind::ConstantNumber(_) => ValueType::Number,
            ExpressionKind::ConstantBoolean(_) => ValueType::Boolean,
            ExpressionKind::ConstantMove(_) => ValueType::Move,
            ExpressionKind::Health => ValueType::Number,
            ExpressionKind::X => ValueType::Number,
            ExpressionKind::Y => ValueType::Number,
            ExpressionKind::GreaterThan { .. } => ValueType::Boolean,
            ExpressionKind::Equals { .. } => ValueType::Boolean,
            ExpressionKind::AlliedSurroundingTiles => ValueType::Number,
            ExpressionKind::EnemySurroundingTiles => ValueType::Number,
            ExpressionKind::AttackNearestEnemy => ValueType::Move,
            ExpressionKind::MoveToNearestEnemy => ValueType::Move,
            ExpressionKind::DistanceToNearestEnemy => ValueType::Number,
            ExpressionKind::DistanceToNearestAlly => ValueType::Number,
            ExpressionKind::DistanceToCenter => ValueType::Number,
            ExpressionKind::ClosestAllyHealth => ValueType::Number,
            ExpressionKind::ClosestEnemyHealth => ValueType::Number
        }
    }

    fn generate_move_expression<RAND: rand::Rng>(rng: &mut RAND) -> ExpressionKind {
        if rng.gen_bool(0.5) {
            return [
                ExpressionKind::AttackNearestEnemy,
                ExpressionKind::MoveToNearestEnemy,
            ][rng.gen_range(0..2)]
            .clone();
        }

        let direction = [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ]
        .choose(rng)
        .unwrap();

        if rng.gen_bool(0.75) {
            return ExpressionKind::ConstantMove(Move::Move(*direction));
        } else {
            return ExpressionKind::ConstantMove(Move::Attack(*direction));
        }
    }

    fn generate_integer_expression<RAND: rand::Rng>(
        rng: &mut RAND,
        range: Option<std::ops::Range<i32>>,
    ) -> ExpressionKind {
        if let Some(r) = range {
            return ExpressionKind::ConstantNumber(rng.gen_range(r));
        }

        return [
            ExpressionKind::DistanceToCenter,
            ExpressionKind::DistanceToNearestAlly,
            ExpressionKind::DistanceToNearestEnemy,
            ExpressionKind::AlliedSurroundingTiles,
            ExpressionKind::EnemySurroundingTiles,
            ExpressionKind::Health,
            ExpressionKind::X,
            ExpressionKind::Y,
        ]
        .choose(rng)
        .unwrap()
        .clone();
    }

    fn generate_boolean_expression<RAND: rand::Rng>(rng: &mut RAND) -> ExpressionKind {
        let left = Expression::new_box(Self::generate_integer_expression(rng, None)
        );
        let right = Expression::new_box(
            Self::generate_integer_expression(rng, left.kind.get_range()),
        );

        if rng.gen_bool(0.1) {
            return ExpressionKind::Equals { left, right };
        } else {
            return ExpressionKind::GreaterThan { left, right };
        }
    }

    pub fn mutate<RAND: rand::Rng>(
        &mut self,
        rng: &mut RAND,
        times_used: usize,
        ignore_sanity_checks: bool,
    ) {
        if (ignore_sanity_checks || times_used > 0) && rng.gen_bool(0.2) {
            let right = match self.get_type() {
                ValueType::Boolean => Self::generate_boolean_expression(rng),
                ValueType::Number => Self::generate_integer_expression(rng, self.get_range()),
                ValueType::Move => Self::generate_move_expression(rng),
            };

            let condition = Self::generate_boolean_expression(rng);

            *self = ExpressionKind::If {
                then: Expression::new_box(self.clone()),
                otherwise: Expression::new_box(right),
                condition: Expression::new_box(condition),
            };
            return;
        }
        match self {
            ExpressionKind::ConstantNumber(t) => {
                *t += rng.sample(rand::distributions::Uniform::new(-1, 2));
            }
            ExpressionKind::If {
                condition: cond,
                then,
                otherwise,
            } => {
                if !ignore_sanity_checks
                    && (then.times_used == 0 || otherwise.times_used == 0)
                    && rng.gen_bool(0.1)
                {
                    *self = if then.times_used == 0 {
                        otherwise.kind.clone()
                    } else {
                        then.kind.clone()
                    }
                } else if !ignore_sanity_checks
                    && (then.times_used == 0 || otherwise.times_used == 0)
                {
                    cond.mutate(rng, ignore_sanity_checks)
                } else if rng.gen_bool(0.5) && then.times_used > 0 {
                    then.mutate(rng, ignore_sanity_checks);
                } else {
                    otherwise.mutate(rng, ignore_sanity_checks);
                }
            }
            ExpressionKind::ConstantBoolean(b) => *b = !*b,
            ExpressionKind::ConstantMove(_) => *self = Self::generate_move_expression(rng),
            ExpressionKind::Health => *self = [ExpressionKind::ClosestEnemyHealth, ExpressionKind::ClosestAllyHealth].choose(rng).unwrap().clone(),
            ExpressionKind::X => *self = ExpressionKind::Y,
            ExpressionKind::Y => *self = ExpressionKind::X,
            ExpressionKind::GreaterThan { left, right } => {
                if rng.gen_bool(0.5) {
                    left.mutate(rng, ignore_sanity_checks)
                } else {
                    right.mutate(rng, ignore_sanity_checks)
                }
            }
            ExpressionKind::Equals { left, right } => {
                if rng.gen_bool(0.5) {
                    left.mutate(rng, ignore_sanity_checks)
                } else {
                    right.mutate(rng, ignore_sanity_checks)
                }
            }
            ExpressionKind::AlliedSurroundingTiles => *self = ExpressionKind::EnemySurroundingTiles,
            ExpressionKind::EnemySurroundingTiles => *self = ExpressionKind::AlliedSurroundingTiles,
            ExpressionKind::AttackNearestEnemy => {
                if rng.gen_bool(0.05) {
                    *self = ExpressionKind::MoveToNearestEnemy
                }
            }
            ExpressionKind::MoveToNearestEnemy => {
                if rng.gen_bool(0.05) {
                    *self = ExpressionKind::AttackNearestEnemy
                }
            }
            ExpressionKind::DistanceToNearestEnemy => (),
            ExpressionKind::DistanceToNearestAlly => {
                if rng.gen_bool(0.05) {
                    *self = ExpressionKind::DistanceToCenter
                }
            }
            ExpressionKind::DistanceToCenter => {
                if rng.gen_bool(0.05) {
                    *self = ExpressionKind::DistanceToNearestAlly
                }
            }
            ExpressionKind::ClosestEnemyHealth => *self = [ExpressionKind::Health, ExpressionKind::ClosestAllyHealth].choose(rng).unwrap().clone(),
            ExpressionKind::ClosestAllyHealth => *self = [ExpressionKind::Health, ExpressionKind::ClosestEnemyHealth].choose(rng).unwrap().clone(),
        }
    }

    fn eval(&mut self, input: &logic::ProgramInput, id: Id, unit: &Unit) -> Result<Value, ()> {
        fn get_surrounding_tiles<'a>(
            input: &'a logic::ProgramInput,
            coords: logic::Coords,
        ) -> impl Iterator<Item = &'a logic::Obj> {
            [(0, 1), (-1, 0), (1, 0), (0, -1)]
                .into_iter()
                .flat_map(move |(dx, dy)| {
                    input.state.grid.get(&Coords(
                        coords.0.wrapping_add_signed(dx),
                        coords.1.wrapping_add_signed(dy),
                    ))
                })
                .flat_map(|id| input.state.objs.get(id))
        }

        fn find_nearest_unit_of_team<'a>(
            input: &'a logic::ProgramInput,
            coords: Coords,
            team: Team,
        ) -> Option<&'a logic::Obj> {
            input
                .state
                .teams
                .get(&team)?
                .iter()
                .flat_map(|d| input.state.objs.get(d))
                .min_by_key(|m| m.0.coords.distance(coords))
        }

        let coords = input.state.objs.get(&id).unwrap().coords();

        match self {
            ExpressionKind::If {
                condition,
                then,
                otherwise,
            } => {
                let condition_result = condition.eval(input, id, unit)?;
                match condition_result {
                    Value::Boolean(true) => then.eval(input, id, unit),
                    Value::Boolean(false) => otherwise.eval(input, id, unit),
                    _ => Err(()),
                }
            }
            ExpressionKind::ConstantNumber(n) => Ok(Value::Number(*n)),
            ExpressionKind::ConstantBoolean(b) => Ok(Value::Boolean(*b)),
            ExpressionKind::ConstantMove(m) => Ok(Value::Move(*m)),
            ExpressionKind::Health => Ok(Value::Number(unit.health as i32)),
            ExpressionKind::X => Ok(Value::Number(
                input.state.objs.get(&id).ok_or(())?.0.coords.0 as i32,
            )),
            ExpressionKind::Y => Ok(Value::Number(
                input.state.objs.get(&id).ok_or(())?.0.coords.1 as i32,
            )),
            ExpressionKind::GreaterThan { left, right } => {
                match (left.eval(input, id, unit)?, right.eval(input, id, unit)?) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a > b)),
                    _ => Err(()),
                }
            }
            ExpressionKind::Equals { left, right } => {
                match (left.eval(input, id, unit)?, right.eval(input, id, unit)?) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Boolean(a == b)),
                    (Value::Move(a), Value::Move(b)) => Ok(Value::Boolean(a == b)),
                    _ => Err(()),
                }
            }
            ExpressionKind::AlliedSurroundingTiles => {
                let surrounding_allies = get_surrounding_tiles(input, coords)
                    .filter(|t| match t.1 {
                        ObjDetails::Unit(Unit { team, .. }) => team == input.team,
                        _ => false,
                    })
                    .count();
                return Ok(Value::Number(surrounding_allies as i32));
            }
            ExpressionKind::EnemySurroundingTiles => {
                let surrounding_allies = get_surrounding_tiles(input, coords)
                    .filter(|t| match t.1 {
                        ObjDetails::Unit(Unit { team, .. }) => team != input.team,
                        _ => false,
                    })
                    .count();
                return Ok(Value::Number(surrounding_allies as i32));
            }
            ExpressionKind::AttackNearestEnemy => {
                let nearest_enemy = find_nearest_unit_of_team(
                    input,
                    coords,
                    match input.team {
                        Team::Red => Team::Blue,
                        Team::Blue => Team::Red,
                    },
                )
                .map(|k| coords.direction(k.coords()))
                .unwrap_or(Direction::East);

                return Ok(Value::Move(Move::Attack(nearest_enemy)));
            }
            ExpressionKind::MoveToNearestEnemy => {
                let nearest_enemy = find_nearest_unit_of_team(
                    input,
                    coords,
                    match input.team {
                        Team::Red => Team::Blue,
                        Team::Blue => Team::Red,
                    },
                )
                .map(|k| coords.direction(k.coords()))
                .unwrap_or(Direction::East);

                return Ok(Value::Move(Move::Move(nearest_enemy)));
            }
            ExpressionKind::DistanceToNearestEnemy => {
                let nearest_enemy = find_nearest_unit_of_team(
                    input,
                    coords,
                    match input.team {
                        Team::Red => Team::Blue,
                        Team::Blue => Team::Red,
                    },
                )
                .map(|k| k.0.coords.distance(coords))
                .unwrap_or(99) as i32;

                return Ok(Value::Number(nearest_enemy));
            }
            ExpressionKind::DistanceToNearestAlly => {
                let nearest_enemy = find_nearest_unit_of_team(input, coords, input.team)
                    .map(|k| k.0.coords.distance(coords))
                    .unwrap_or(99) as i32;

                return Ok(Value::Number(nearest_enemy));
            }
            ExpressionKind::DistanceToCenter => {
                Ok(Value::Number(coords.distance(Coords(9, 9)) as i32))
            }
            ExpressionKind::ClosestEnemyHealth => {
                let closest_enemy_health = find_nearest_unit_of_team(input, coords, input.team.opposite())
                    .and_then(|k| match k.1 {
                        ObjDetails::Unit(Unit { health, ..}) => Some(health),
                        _ => None
                    })
                    .unwrap_or(0) as i32;

                return Ok(Value::Number(closest_enemy_health));
            },
            ExpressionKind::ClosestAllyHealth => {
                let closest_ally_health = find_nearest_unit_of_team(input, coords, input.team)
                    .and_then(|k| match k.1 {
                        ObjDetails::Unit(Unit { health, ..}) => Some(health),
                        _ => None
                    })
                    .unwrap_or(0) as i32;

                return Ok(Value::Number(closest_ally_health));
            },
        }
    }

    pub fn similarity(&self, other: &Self) -> f32 {
        if self == other {
            return 1.0;
        }

        match (self, other) {
            (
                ExpressionKind::If {
                    condition: condition_a,
                    then: then_a,
                    otherwise: otherwise_a,
                },
                ExpressionKind::If {
                    condition: condition_b,
                    then: then_b,
                    otherwise: otherwise_b,
                },
            ) => {
                0.5 + condition_a.kind.similarity(&condition_b.kind) * 0.25
                    + then_a.kind.similarity(&then_b.kind)
                    + 0.25 * otherwise_a.kind.similarity(&otherwise_b.kind)
            }
            (ExpressionKind::ConstantNumber(a), ExpressionKind::ConstantNumber(b)) => {
                if a.abs_diff(*b) <= 1 {
                    0.75
                } else {
                    0.55
                }
            }
            (
                ExpressionKind::Equals {
                    left: left_a,
                    right: _right_a,
                },
                ExpressionKind::Equals {
                    left: left_b,
                    right: right_b,
                },
            ) => {
                0.5 + 0.25
                    + left_a.kind.similarity(&left_b.kind)
                    + 0.25
                    + right_b.kind.similarity(&right_b.kind)
            }
            (
                ExpressionKind::GreaterThan {
                    left: left_a,
                    right: _right_a,
                },
                ExpressionKind::GreaterThan {
                    left: left_b,
                    right: right_b,
                },
            ) => {
                0.5 + 0.25
                    + left_a.kind.similarity(&left_b.kind)
                    + 0.25
                    + right_b.kind.similarity(&right_b.kind)
            }
            _ => 0.0,
        }
    }

    fn get_range(&self) -> Option<std::ops::Range<i32>> {
        match self {
            ExpressionKind::Health => Some(1..11),
            ExpressionKind::X => Some(0..20),
            ExpressionKind::Y => Some(0..20),
            ExpressionKind::ConstantNumber(m) => Some(*m..m + 1),
            ExpressionKind::DistanceToCenter
            | ExpressionKind::DistanceToNearestAlly
            | ExpressionKind::DistanceToNearestEnemy => Some(0..10),
            ExpressionKind::AlliedSurroundingTiles | ExpressionKind::EnemySurroundingTiles => {
                Some(0..5)
            }
            _ => None,
        }
    }

    pub fn clear_times_used(&mut self) {
        match self {
            ExpressionKind::If {
                condition,
                then,
                otherwise,
            } => {
                condition.clear_times_used();
                then.clear_times_used();
                otherwise.clear_times_used();
            }
            ExpressionKind::Equals { left, right } => {
                left.clear_times_used();
                right.clear_times_used();
            }
            ExpressionKind::GreaterThan { left, right } => {
                left.clear_times_used();
                right.clear_times_used();
            }
            _ => (),
        }
    }

    fn simplify(self) -> ExpressionKind {
        match self {
            ExpressionKind::If {
                condition,
                then,
                otherwise,
            } => {
                if then == otherwise {
                    then.kind.simplify()
                } else {
                    match condition.kind {
                        ExpressionKind::ConstantBoolean(true) => then.kind.simplify(),
                        ExpressionKind::ConstantBoolean(false) => otherwise.kind.simplify(),
                        _ => ExpressionKind::If {
                            condition: Box::new(condition.simplify()),
                            then: Box::new(then.simplify()),
                            otherwise: Box::new(otherwise.simplify()),
                        },
                    }
                }
            }
            ExpressionKind::Equals { left, right } => {
                if left == right {
                    ExpressionKind::ConstantBoolean(true)
                } else if !left
                    .kind
                    .get_range()
                    .zip(right.kind.get_range())
                    .map(|(a, b)| a.start < b.end && b.start < a.end)
                    .unwrap_or(false)
                {
                    ExpressionKind::ConstantBoolean(false)
                } else {
                    ExpressionKind::Equals {
                        left: Box::new(left.simplify()),
                        right: Box::new(right.simplify()),
                    }
                }
            }
            ExpressionKind::GreaterThan { left, right } => {
                if left.kind == right.kind {
                    ExpressionKind::ConstantBoolean(true)
                } else if left
                    .kind
                    .get_range()
                    .zip(right.kind.get_range())
                    .map(|(a, b)| a.start >= b.end)
                    .unwrap_or(false)
                {
                    ExpressionKind::ConstantBoolean(true)
                } else if left
                    .kind
                    .get_range()
                    .zip(right.kind.get_range())
                    .map(|(a, b)| b.start >= a.end)
                    .unwrap_or(false)
                {
                    ExpressionKind::ConstantBoolean(false)
                } else {
                    match (&left.kind, &right.kind) {
                        (ExpressionKind::ConstantNumber(a), ExpressionKind::ConstantNumber(b)) => {
                            ExpressionKind::ConstantBoolean(a > b)
                        }
                        _ => ExpressionKind::GreaterThan {
                            left: Box::new(left.simplify()),
                            right: Box::new(right.simplify()),
                        },
                    }
                }
            }
            other => other,
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
            ExpressionKind::AlliedSurroundingTiles => {
                write!(f, "friendly_surrounding_tiles(unit.coords, state)")
            }
            ExpressionKind::EnemySurroundingTiles => {
                write!(f, "unsafe_surrounding_tiles(unit.coords, state)")
            }
            ExpressionKind::AttackNearestEnemy => write!(
                f,
                "Action.attack(unit.coords.direction_to(closest_enemy.coords))"
            ),
            ExpressionKind::MoveToNearestEnemy => write!(
                f,
                "Action.move(unit.coords.direction_to(closest_enemy.coords))"
            ),
            ExpressionKind::DistanceToNearestEnemy => {
                write!(f, "closest_enemy.coords.distance_to(unit.coords)")
            }
            ExpressionKind::DistanceToNearestAlly => {
                write!(f, "closest_ally.coords.distance_to(unit.coords)")
            }
            ExpressionKind::DistanceToCenter => write!(f, "Coords(9,9).distance_to(unit.coords)"),
            ExpressionKind::ClosestEnemyHealth => write!(f, "closest_enemy.health"),
            ExpressionKind::ClosestAllyHealth => write!(f, "closest_ally.health"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        let mut moves = BTreeMap::new();

        for &bot in input.state.teams.get(&input.team).unwrap() {
            let result = self.eval(
                &input,
                bot,
                match &input.state.objs.get(&bot).unwrap().1 {
                    ObjDetails::Unit(k) => &k,
                    _ => panic!("unexpected unit type"),
                },
            );

            let action = result
                .map(|result| match result {
                    Value::Move(m) => Some(match m {
                        Move::Attack(direction) => logic::Action {
                            type_: ActionType::Attack,
                            direction: direction.into(),
                        },
                        Move::Move(direction) => logic::Action {
                            type_: ActionType::Move,
                            direction: direction.into(),
                        },
                    }),
                    _ => panic!("Expected expression to return an action"),
                })
                .map_err(|_k| {
                    eprint!("Bot errored");
                    logic::Error {
                        summary: format!(""),
                        details: None,
                        loc: None,
                    }
                });
            moves.insert(bot, action);
        }

        Ok(logic::ProgramOutput {
            robot_actions: moves,
            logs: vec![],
            debug_inspect_tables: BTreeMap::new(),
            debug_locate_queries: vec![],
        })
    }
}

// #[async_trait::async_trait]
// impl RobotRunner for &mut Expression {
//     async fn run(&mut self, input: logic::ProgramInput<'_>) -> logic::ProgramResult {
//         (**self).run(input).await
//     }
// }
