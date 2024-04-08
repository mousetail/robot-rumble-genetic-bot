use logic::Team;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Into<logic::Direction> for Direction {
    fn into(self) -> logic::Direction {
        match self {
            Direction::East => logic::Direction::East,
            Direction::West => logic::Direction::West,
            Direction::North => logic::Direction::North,
            Direction::South => logic::Direction::South,
        }
    }
}

pub trait CoordsExt {
    fn distance(self, other: Self) -> usize;
    fn direction(self, to: Self) -> Direction;
}

impl CoordsExt for logic::Coords {
    fn distance(self, b: Self) -> usize {
        self.0.abs_diff(b.1) + self.1.abs_diff(b.1)
    }

    fn direction(self, to: Self) -> Direction {
        let diff = (self.0 as f32 - to.0 as f32, self.1 as f32 - to.1 as f32);
        let angle = (diff.0).atan2(diff.1);

        if angle.abs() < std::f32::consts::FRAC_PI_4 {
            return Direction::West;
        } else if (angle - std::f32::consts::FRAC_PI_2).abs() <= std::f32::consts::FRAC_PI_4 {
            return Direction::South;
        } else if (angle + std::f32::consts::FRAC_PI_2).abs() <= std::f32::consts::FRAC_PI_4 {
            return Direction::North;
        } else {
            return Direction::East;
        }
    }
}

pub trait TeamExt {
    fn opposite(self) -> Self;
}

impl TeamExt for logic::Team {
    fn opposite(self) -> Self {
        match self {
            Team::Red => Team::Blue,
            Team::Blue => Team::Red
        }
    }
}
