pub mod biome;
pub mod block;
pub mod chonk;
pub mod map;
pub mod sprite;
pub mod structure;

// Reexports
pub use self::{
    biome::BiomeKind,
    block::{Block, BlockKind},
    map::MapSizeLg,
    sprite::SpriteKind,
    structure::Structure,
};
use roots::find_roots_cubic;
use serde::{Deserialize, Serialize};

use crate::{vol::RectVolSize, volumes::vol_grid_2d::VolGrid2d};
use vek::*;

// TerrainChunkSize

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainChunkSize;

/// Base two logarithm of the number of blocks along either horizontal axis of
/// a chunk.
///
/// NOTE: (1 << CHUNK_SIZE_LG) is guaranteed to fit in a u32.
///
/// NOTE: A lot of code assumes that the two dimensions are equal, so we make it
/// explicit here.
///
/// NOTE: It is highly unlikely that a value greater than 5 will work, as many
/// frontend optimizations rely on being able to pack chunk horizontal
/// dimensions into 5 bits each.
pub const TERRAIN_CHUNK_BLOCKS_LG: u32 = 5;

impl RectVolSize for TerrainChunkSize {
    const RECT_SIZE: Vec2<u32> = Vec2 {
        x: (1 << TERRAIN_CHUNK_BLOCKS_LG),
        y: (1 << TERRAIN_CHUNK_BLOCKS_LG),
    };
}

// TerrainChunkMeta

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainChunkMeta {
    name: Option<String>,
    biome: BiomeKind,
}

impl TerrainChunkMeta {
    pub fn new(name: Option<String>, biome: BiomeKind) -> Self { Self { name, biome } }

    pub fn void() -> Self {
        Self {
            name: None,
            biome: BiomeKind::Void,
        }
    }

    pub fn name(&self) -> &str { self.name.as_deref().unwrap_or("Wilderness") }

    pub fn biome(&self) -> BiomeKind { self.biome }
}

// Terrain type aliases

pub type TerrainChunk = chonk::Chonk<Block, TerrainChunkSize, TerrainChunkMeta>;
pub type TerrainGrid = VolGrid2d<TerrainChunk>;

// Terrain helper functions used across multiple crates.

/// Computes the position Vec2 of a SimChunk from an index, where the index was
/// generated by uniform_noise.
///
/// NOTE: Dimensions obey constraints on [map::MapConfig::map_size_lg].
#[inline(always)]
pub fn uniform_idx_as_vec2(map_size_lg: MapSizeLg, idx: usize) -> Vec2<i32> {
    let x_mask = (1 << map_size_lg.vec().x) - 1;
    Vec2::new((idx & x_mask) as i32, (idx >> map_size_lg.vec().x) as i32)
}

/// Computes the index of a Vec2 of a SimChunk from a position, where the index
/// is generated by uniform_noise.  NOTE: Both components of idx should be
/// in-bounds!
#[inline(always)]
pub fn vec2_as_uniform_idx(map_size_lg: MapSizeLg, idx: Vec2<i32>) -> usize {
    ((idx.y as usize) << map_size_lg.vec().x) | idx.x as usize
}

// NOTE: want to keep this such that the chunk index is in ascending order!
pub const NEIGHBOR_DELTA: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Iterate through all cells adjacent to a chunk.
#[inline(always)]
pub fn neighbors(map_size_lg: MapSizeLg, posi: usize) -> impl Clone + Iterator<Item = usize> {
    let pos = uniform_idx_as_vec2(map_size_lg, posi);
    let world_size = map_size_lg.chunks();
    NEIGHBOR_DELTA
        .iter()
        .map(move |&(x, y)| Vec2::new(pos.x + x, pos.y + y))
        .filter(move |pos| {
            pos.x >= 0 && pos.y >= 0 && pos.x < world_size.x as i32 && pos.y < world_size.y as i32
        })
        .map(move |pos| vec2_as_uniform_idx(map_size_lg, pos))
}

pub fn river_spline_coeffs(
    // _sim: &WorldSim,
    chunk_pos: Vec2<f64>,
    spline_derivative: Vec2<f32>,
    downhill_pos: Vec2<f64>,
) -> Vec3<Vec2<f64>> {
    let dxy = downhill_pos - chunk_pos;
    // Since all splines have been precomputed, we don't have to do that much work
    // to evaluate the spline.  The spline is just ax^2 + bx + c = 0, where
    //
    // a = dxy - chunk.river.spline_derivative
    // b = chunk.river.spline_derivative
    // c = chunk_pos
    let spline_derivative = spline_derivative.map(|e| e as f64);
    Vec3::new(dxy - spline_derivative, spline_derivative, chunk_pos)
}

/// Find the nearest point from a quadratic spline to this point (in terms of t,
/// the "distance along the curve" by which our spline is parameterized).  Note
/// that if t < 0.0 or t >= 1.0, we probably shouldn't be considered "on the
/// curve"... hopefully this works out okay and gives us what we want (a
/// river that extends outwards tangent to a quadratic curve, with width
/// configured by distance along the line).
#[allow(clippy::let_and_return)] // TODO: Pending review in #587
#[allow(clippy::many_single_char_names)]
pub fn quadratic_nearest_point(
    spline: &Vec3<Vec2<f64>>,
    point: Vec2<f64>,
) -> Option<(f64, Vec2<f64>, f64)> {
    let a = spline.z.x;
    let b = spline.y.x;
    let c = spline.x.x;
    let d = point.x;
    let e = spline.z.y;
    let f = spline.y.y;
    let g = spline.x.y;
    let h = point.y;
    // This is equivalent to solving the following cubic equation (derivation is a
    // bit annoying):
    //
    // A = 2(c^2 + g^2)
    // B = 3(b * c + g * f)
    // C = ((a - d) * 2 * c + b^2 + (e - h) * 2 * g + f^2)
    // D = ((a - d) * b + (e - h) * f)
    //
    // Ax?? + Bx?? + Cx + D = 0
    //
    // Once solved, this yield up to three possible values for t (reflecting minimal
    // and maximal values).  We should choose the minimal such real value with t
    // between 0.0 and 1.0.  If we fall outside those bounds, then we are
    // outside the spline and return None.
    let a_ = (c * c + g * g) * 2.0;
    let b_ = (b * c + g * f) * 3.0;
    let a_d = a - d;
    let e_h = e - h;
    let c_ = a_d * c * 2.0 + b * b + e_h * g * 2.0 + f * f;
    let d_ = a_d * b + e_h * f;
    let roots = find_roots_cubic(a_, b_, c_, d_);
    let roots = roots.as_ref();

    let min_root = roots
        .iter()
        .copied()
        .filter_map(|root| {
            let river_point = spline.x * root * root + spline.y * root + spline.z;
            let river_zero = spline.z;
            let river_one = spline.x + spline.y + spline.z;
            if root > 0.0 && root < 1.0 {
                Some((root, river_point))
            } else if river_point.distance_squared(river_zero) < 0.5 {
                Some((root, /*river_point*/ river_zero))
            } else if river_point.distance_squared(river_one) < 0.5 {
                Some((root, /*river_point*/ river_one))
            } else {
                None
            }
        })
        .map(|(root, river_point)| {
            let river_distance = river_point.distance_squared(point);
            (root, river_point, river_distance)
        })
        // In the (unlikely?) case that distances are equal, prefer the earliest point along the
        // river.
        .min_by(|&(ap, _, a), &(bp, _, b)| {
            (a, ap < 0.0 || ap > 1.0, ap)
                .partial_cmp(&(b, bp < 0.0 || bp > 1.0, bp))
                .unwrap()
        });
    min_root
}
