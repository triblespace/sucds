//! Compressed integer sequence using Directly Addressable Codes (DACs) in a simple bytewise scheme.
#![cfg(target_pointer_width = "64")]

use std::convert::TryFrom;

use anyhow::{anyhow, Result};
use num_traits::ToPrimitive;

use crate::bit_vector::bit_vector::BitVectorBuilder;
use crate::bit_vector::rank9sel::inner::Rank9SelIndex;
use crate::bit_vector::{self, BitVector, Rank};
use crate::int_vectors::{Access, Build, NumVals};
use crate::utils;
use anybytes::{Bytes, View};

const LEVEL_WIDTH: usize = 8;
const LEVEL_MASK: usize = (1 << LEVEL_WIDTH) - 1;

/// Compressed integer sequence using Directly Addressable Codes (DACs) in a simple bytewise scheme.
///
/// DACs are a compact representation of an integer sequence consisting of many small values.
/// [`DacsByte`] stores each level as a zero-copy [`View<[u8]>`] to avoid extra copying.
///
/// # Memory complexity
///
/// $`\textrm{DAC}(A) + o(\textrm{DAC}(A)/b) + O(\lg u)`$ bits where
///
/// - $`u`$ is the maximum value plus 1,
/// - $`b`$ is the length in bits assigned for each level with DACs (here $`b = 8`$), and
/// - $`\textrm{DAC}(A)`$ is the length in bits of the encoded sequence from an original sequence $`A`$ with DACs.
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use jerky::int_vectors::{DacsByte, Access};
///
/// let seq = DacsByte::from_slice(&[5, 0, 100000, 334])?;
///
/// assert_eq!(seq.access(0), Some(5));
/// assert_eq!(seq.access(1), Some(0));
/// assert_eq!(seq.access(2), Some(100000));
/// assert_eq!(seq.access(3), Some(334));
///
/// assert_eq!(seq.len(), 4);
/// assert_eq!(seq.num_levels(), 3);
/// # Ok(())
/// # }
/// ```
///
/// # References
///
/// - N. R. Brisaboa, S. Ladra, and G. Navarro, "DACs: Bringing direct access to variable-length
///   codes." Information Processing & Management, 49(1), 392-404, 2013.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DacsByte {
    data: Vec<View<[u8]>>,
    flags: Vec<BitVector<Rank9SelIndex>>,
}

impl DacsByte {
    /// Builds DACs by assigning 8 bits to represent each level.
    ///
    /// # Arguments
    ///
    /// - `vals`: Slice of integers to be stored.
    ///
    /// # Errors
    ///
    /// An error is returned if `vals` contains an integer that cannot be cast to [`usize`].
    pub fn from_slice<T>(vals: &[T]) -> Result<Self>
    where
        T: ToPrimitive,
    {
        if vals.is_empty() {
            return Ok(Self::default());
        }

        let mut maxv = 0;
        for x in vals {
            maxv =
                maxv.max(x.to_usize().ok_or_else(|| {
                    anyhow!("vals must consist only of values castable into usize.")
                })?);
        }
        let num_bits = utils::needed_bits(maxv);
        let num_levels = utils::ceiled_divide(num_bits, LEVEL_WIDTH);
        assert_ne!(num_levels, 0);

        if num_levels == 1 {
            let data: Vec<_> = vals
                .iter()
                .map(|x| u8::try_from(x.to_usize().unwrap()).unwrap())
                .collect();
            return Ok(Self {
                data: vec![Bytes::from_source(data).view::<[u8]>().unwrap()],
                flags: vec![],
            });
        }

        let mut data = vec![vec![]; num_levels];
        let mut flags = vec![BitVectorBuilder::new(); num_levels - 1];

        for x in vals {
            let mut x = x.to_usize().unwrap();
            for j in 0..num_levels {
                data[j].push(u8::try_from(x & LEVEL_MASK).unwrap());
                x >>= LEVEL_WIDTH;
                if j == num_levels - 1 {
                    assert_eq!(x, 0);
                    break;
                } else if x == 0 {
                    flags[j].push_bit(false);
                    break;
                }
                flags[j].push_bit(true);
            }
        }

        let flags = flags
            .into_iter()
            .map(|bvb| bvb.freeze::<Rank9SelIndex<true, true>>())
            .collect();
        let data = data
            .into_iter()
            .map(|v| Bytes::from_source(v).view::<[u8]>().unwrap())
            .collect();
        Ok(Self { data, flags })
    }

    /// Creates an iterator for enumerating integers.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use jerky::int_vectors::DacsByte;
    ///
    /// let seq = DacsByte::from_slice(&[5, 0, 100000, 334])?;
    /// let mut it = seq.iter();
    ///
    /// assert_eq!(it.next(), Some(5));
    /// assert_eq!(it.next(), Some(0));
    /// assert_eq!(it.next(), Some(100000));
    /// assert_eq!(it.next(), Some(334));
    /// assert_eq!(it.next(), None);
    /// # Ok(())
    /// # }
    /// ```
    pub const fn iter(&self) -> Iter {
        Iter::new(self)
    }

    /// Gets the number of integers.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.data[0].len()
    }

    /// Checks if the vector is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Gets the number of levels.
    #[inline(always)]
    pub fn num_levels(&self) -> usize {
        self.data.len()
    }

    /// Gets the number of bits for each level.
    #[inline(always)]
    pub fn widths(&self) -> Vec<usize> {
        self.data.iter().map(|_| LEVEL_WIDTH).collect()
    }
}

impl Default for DacsByte {
    fn default() -> Self {
        Self {
            // Needs a single level at least.
            data: vec![Bytes::empty().view::<[u8]>().unwrap()],
            flags: vec![],
        }
    }
}

impl Build for DacsByte {
    /// Creates a new vector from a slice of integers `vals`.
    ///
    /// This just calls [`Self::from_slice()`]. See the documentation.
    fn build_from_slice<T>(vals: &[T]) -> Result<Self>
    where
        T: ToPrimitive,
        Self: Sized,
    {
        Self::from_slice(vals)
    }
}

impl NumVals for DacsByte {
    /// Returns the number of integers stored (just wrapping [`Self::len()`]).
    fn num_vals(&self) -> usize {
        self.len()
    }
}

impl Access for DacsByte {
    /// Returns the `pos`-th integer, or [`None`] if out of bounds.
    ///
    /// # Complexity
    ///
    /// $`O( \ell_{pos} )`$ where $`\ell_{pos}`$ is the number of levels corresponding to
    /// the `pos`-th integer.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use jerky::int_vectors::{DacsByte, Access};
    ///
    /// let seq = DacsByte::from_slice(&[5, 999, 334])?;
    ///
    /// assert_eq!(seq.access(0), Some(5));
    /// assert_eq!(seq.access(1), Some(999));
    /// assert_eq!(seq.access(2), Some(334));
    /// assert_eq!(seq.access(3), None);
    /// # Ok(())
    /// # }
    /// ```
    fn access(&self, mut pos: usize) -> Option<usize> {
        if self.len() <= pos {
            return None;
        }
        let mut x = 0;
        for j in 0..self.num_levels() {
            x |= usize::from(self.data[j][pos]) << (j * LEVEL_WIDTH);
            if j == self.num_levels() - 1
                || !bit_vector::Access::access(&self.flags[j], pos).unwrap()
            {
                break;
            }
            pos = self.flags[j].rank1(pos).unwrap();
        }
        Some(x)
    }
}

/// Iterator for enumerating integers, created by [`DacsByte::iter()`].
pub struct Iter<'a> {
    seq: &'a DacsByte,
    pos: usize,
}

impl<'a> Iter<'a> {
    /// Creates a new iterator.
    pub const fn new(seq: &'a DacsByte) -> Self {
        Self { seq, pos: 0 }
    }
}

impl Iterator for Iter<'_> {
    type Item = usize;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.seq.len() {
            let x = self.seq.access(self.pos).unwrap();
            self.pos += 1;
            Some(x)
        } else {
            None
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.seq.len(), Some(self.seq.len()))
    }
}

impl DacsByte {
    /// Returns the number of bytes required for the old copy-based serialization.
    pub fn size_in_bytes(&self) -> usize {
        std::mem::size_of::<usize>()
            + self
                .data
                .iter()
                .map(|v| std::mem::size_of::<usize>() + v.len())
                .sum::<usize>()
            + std::mem::size_of::<usize>()
            + self
                .flags
                .iter()
                .map(|f| f.data.size_in_bytes() + f.index.size_in_bytes())
                .sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anybytes::Bytes;

    #[test]
    fn test_basic() {
        let seq = DacsByte::from_slice(&[0xFFFF, 0xFF, 0xF, 0xFFFFF, 0xF]).unwrap();

        let expected = vec![
            Bytes::from_source(vec![0xFFu8, 0xFF, 0xF, 0xFF, 0xF])
                .view::<[u8]>()
                .unwrap(),
            Bytes::from_source(vec![0xFFu8, 0xFF])
                .view::<[u8]>()
                .unwrap(),
            Bytes::from_source(vec![0xFu8]).view::<[u8]>().unwrap(),
        ];
        assert_eq!(seq.data, expected);

        let mut b = BitVectorBuilder::new();
        b.extend_bits([true, false, false, true, false]);
        let f0 = b.freeze::<Rank9SelIndex<true, true>>();
        let mut b = BitVectorBuilder::new();
        b.extend_bits([false, true]);
        let f1 = b.freeze::<Rank9SelIndex<true, true>>();
        assert_eq!(seq.flags, vec![f0, f1]);

        assert!(!seq.is_empty());
        assert_eq!(seq.len(), 5);
        assert_eq!(seq.num_levels(), 3);
        assert_eq!(seq.widths(), vec![LEVEL_WIDTH, LEVEL_WIDTH, LEVEL_WIDTH]);

        assert_eq!(seq.access(0), Some(0xFFFF));
        assert_eq!(seq.access(1), Some(0xFF));
        assert_eq!(seq.access(2), Some(0xF));
        assert_eq!(seq.access(3), Some(0xFFFFF));
        assert_eq!(seq.access(4), Some(0xF));
    }

    #[test]
    fn test_empty() {
        let seq = DacsByte::from_slice::<usize>(&[]).unwrap();
        assert!(seq.is_empty());
        assert_eq!(seq.len(), 0);
        assert_eq!(seq.num_levels(), 1);
        assert_eq!(seq.widths(), vec![LEVEL_WIDTH]);
    }

    #[test]
    fn test_all_zeros() {
        let seq = DacsByte::from_slice(&[0, 0, 0, 0]).unwrap();
        assert!(!seq.is_empty());
        assert_eq!(seq.len(), 4);
        assert_eq!(seq.num_levels(), 1);
        assert_eq!(seq.widths(), vec![LEVEL_WIDTH]);
        assert_eq!(seq.access(0), Some(0));
        assert_eq!(seq.access(1), Some(0));
        assert_eq!(seq.access(2), Some(0));
        assert_eq!(seq.access(3), Some(0));
    }

    #[test]
    fn test_from_slice_uncastable() {
        let e = DacsByte::from_slice(&[u128::MAX]);
        assert_eq!(
            e.err().map(|x| x.to_string()),
            Some("vals must consist only of values castable into usize.".to_string())
        );
    }
}
