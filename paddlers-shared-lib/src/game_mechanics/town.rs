pub const TOWN_X: usize = 23;
pub const TOWN_Y: usize = 13;
pub const TOWN_LANE_Y: usize = 6;

#[derive(Debug)]
pub struct TownMap(pub [[TownTileType; TOWN_Y]; TOWN_X]);
pub type TileIndex = (usize, usize);

#[derive(PartialEq, Eq,Clone, Copy, Debug)]
pub enum TownTileType {
    EMPTY,
    BUILDING,
    LANE,
}

impl TownMap {
    pub fn basic_map() -> TownMap {
        let mut map = [[TownTileType::EMPTY; TOWN_Y]; TOWN_X];
        for x in 0..TOWN_X {
            for y in 0..TOWN_Y {
                if y == TOWN_LANE_Y {
                    map[x][y] = TownTileType::LANE;
                }
            }
        }
        TownMap(map)
    }

    pub fn tile_type(&self, index: TileIndex) -> Option<&TownTileType> {
        self.0.get(index.0).and_then(|m| m.get(index.1))
    }
    pub fn tile_type_mut(&mut self, index: TileIndex) -> Option<&mut TownTileType> {
        self.0.get_mut(index.0).and_then(|m| m.get_mut(index.1))
    }
}


impl TownTileType {
    pub fn is_buildable(&self) -> bool {
        match self {
            TownTileType::EMPTY 
                => true,
            TownTileType::BUILDING 
            | TownTileType::LANE 
                => false,
        }
    }
    pub fn is_walkable(&self) -> bool {
        match self {
            TownTileType::EMPTY 
            | TownTileType::LANE 
                => true,
            TownTileType::BUILDING 
                => false,
        }
    }
}


use std::ops::{Index, IndexMut};
impl Index<TileIndex> for TownMap {
    type Output = TownTileType;

    fn index(&self, idx: TileIndex)-> &Self::Output {
        &self.0[idx.0][idx.1]
    }
}
impl IndexMut<TileIndex> for TownMap {
    fn index_mut(&mut self, idx: TileIndex)-> &mut Self::Output {
        &mut self.0[idx.0][idx.1]
    }
}

pub fn distance2(a: TileIndex, b: TileIndex) -> f32 {
    let x = (a.0 as i32 - b.0 as i32) as f32;
    let y = (a.1 as i32 - b.1 as i32) as f32;
    x * x + y * y
}