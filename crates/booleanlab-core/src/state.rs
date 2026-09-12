use core::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitState {
    words: Vec<u64>,
    width: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateError {
    ZeroWidth,
    BitOutOfRange { bit: usize, width: usize },
    WidthMismatch { left: usize, right: usize },
}

impl BitState {
    pub fn zero(width: usize) -> Result<Self, StateError> {
        if width == 0 {
            return Err(StateError::ZeroWidth);
        }
        Ok(Self {
            words: vec![0; width.div_ceil(64)],
            width,
        })
    }

    pub fn from_bools(bits: &[bool]) -> Result<Self, StateError> {
        let mut state = Self::zero(bits.len())?;
        for (index, &value) in bits.iter().enumerate() {
            state.set(index, value)?;
        }
        Ok(state)
    }

    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    #[must_use]
    pub fn words(&self) -> &[u64] {
        &self.words
    }

    pub fn get(&self, bit: usize) -> Result<bool, StateError> {
        self.check(bit)?;
        Ok((self.words[bit / 64] & (1_u64 << (bit % 64))) != 0)
    }

    pub fn set(&mut self, bit: usize, value: bool) -> Result<(), StateError> {
        self.check(bit)?;
        let mask = 1_u64 << (bit % 64);
        let word = &mut self.words[bit / 64];
        if value {
            *word |= mask;
        } else {
            *word &= !mask;
        }
        Ok(())
    }

    pub fn flip(&mut self, bit: usize) -> Result<(), StateError> {
        self.check(bit)?;
        self.words[bit / 64] ^= 1_u64 << (bit % 64);
        Ok(())
    }

    pub fn hamming_distance(&self, other: &Self) -> Result<u32, StateError> {
        if self.width != other.width {
            return Err(StateError::WidthMismatch {
                left: self.width,
                right: other.width,
            });
        }
        Ok(self
            .words
            .iter()
            .zip(&other.words)
            .map(|(left, right)| (left ^ right).count_ones())
            .sum())
    }

    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        (0..self.width).map(|bit| {
            let word = self.words[bit / 64];
            (word & (1_u64 << (bit % 64))) != 0
        })
    }

    fn check(&self, bit: usize) -> Result<(), StateError> {
        if bit >= self.width {
            Err(StateError::BitOutOfRange {
                bit,
                width: self.width,
            })
        } else {
            Ok(())
        }
    }
}

impl fmt::Display for StateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroWidth => write!(formatter, "Boolean state width must be non-zero"),
            Self::BitOutOfRange { bit, width } => {
                write!(formatter, "bit index {bit} is outside state width {width}")
            }
            Self::WidthMismatch { left, right } => {
                write!(formatter, "state width mismatch: {left} != {right}")
            }
        }
    }
}

impl std::error::Error for StateError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_across_word_boundaries() {
        let mut state = BitState::zero(130).unwrap();
        state.set(0, true).unwrap();
        state.set(64, true).unwrap();
        state.set(129, true).unwrap();
        assert!(state.get(0).unwrap());
        assert!(state.get(64).unwrap());
        assert!(state.get(129).unwrap());
        assert_eq!(state.words().len(), 3);
    }

    #[test]
    fn computes_hamming_distance() {
        let left = BitState::from_bools(&[true, false, true, false]).unwrap();
        let right = BitState::from_bools(&[false, false, true, true]).unwrap();
        assert_eq!(left.hamming_distance(&right), Ok(2));
    }
}
