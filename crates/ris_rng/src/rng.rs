use ris_util::error::RisResult;

use crate::pcg::Pcg32;

pub struct Seed(pub [u8; 16]);
pub const CONST_SEED: Seed = Seed([198, 237, 209, 128, 44, 192, 237, 30, 31, 198, 222, 241, 131, 161, 105, 206]);

impl Seed {
    pub fn new() -> RisResult<Self> {
        let now = std::time::SystemTime::now();
        let duration_since_epoch = ris_util::unroll!(
            now.duration_since(std::time::UNIX_EPOCH),
            "failed to get time",
        )?;
        let bytes = duration_since_epoch.as_millis().to_le_bytes();
        let seed = Seed(bytes);

        Ok(seed)
    }
}

pub struct Rng {
    seed: Seed,
    pcg: Pcg32,
}

impl Rng {
    pub fn new(seed: Seed) -> Rng {
        let pcg = Pcg32::new_from_seed(seed.0);
        Rng {
            seed,
            pcg,
        }
    }

    pub fn seed(&self) -> &Seed {
        &self.seed
    }

    pub fn next_u(&mut self) -> u32 {
        self.pcg.next()
    }

    pub fn next_b(&mut self) -> bool {
        (self.next_u() & 1) == 1
    }

    pub fn next_f(&mut self) -> f32 {
        f32::from_bits(0x3F80_0000 | (self.next_u() & 0x7F_FFFF)) - 1.
    }

    pub fn range_f(&mut self, min: f32, max: f32) -> f32 {
        if max <= min {
            if max == min {
                return min;
            } else {
                return f32::NAN;
            }
        }

        let r = (max - min + 1.) * self.next_f() + min;

        if r > max {
            max
        } else {
            r
        }
    }

    pub fn range_i(&mut self, min: i32, max: i32) -> i32 {
        if max <= min {
            if max == min {
                return min;
            } else {
                return i32::MIN;
            }
        }

        let r = (((max - min) as f32) * self.next_f()) as i32 + min;

        if r > max {
            max
        } else {
            r
        }
    }
}
