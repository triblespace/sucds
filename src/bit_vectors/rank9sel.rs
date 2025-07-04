//! Rank/select data structure over bit vectors with Vigna's rank9 and hinted selection techniques.
#![cfg(target_pointer_width = "64")]

pub mod inner;

use anyhow::Result;

use crate::bit_vectors::data::BitVectorData;
use crate::bit_vectors::prelude::*;
use crate::bit_vectors::RawBitVector;
use inner::{Rank9SelIndex, Rank9SelIndexBuilder};

/// Rank/select data structure over bit vectors with Vigna's rank9 and hinted selection techniques.
///
/// This builds rank/select indices on [`RawBitVector`] taking
///
/// - 25% overhead of space for the rank index, and
/// - 3% overhead of space for the select index (together with the rank's overhead).
///
/// # Notes
///
/// In the default configuration, it does not build the select index for faster queries.
/// To accelerate the queries, enable select hints when constructing the structure
/// using [`Build::build_from_bits`] or [`Rank9SelIndexBuilder`].
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use jerky::bit_vectors::{Rank9Sel, Access, Rank, Select, Build};
///
/// let bv = Rank9Sel::build_from_bits(
///     [true, false, false, true],
///     false,
///     true,
///     true,
/// )?;
///
/// assert_eq!(bv.len(), 4);
/// assert_eq!(bv.access(1), Some(false));
///
/// assert_eq!(bv.rank1(1), Some(1));
/// assert_eq!(bv.rank0(1), Some(0));
///
/// assert_eq!(bv.select1(1), Some(3));
/// assert_eq!(bv.select0(0), Some(1));
/// # Ok(())
/// # }
/// ```
///
/// # Credits
///
/// This is a yet another Rust port of [succinct::rs_bit_vector](https://github.com/ot/succinct/blob/master/rs_bit_vector.hpp).
///
/// # References
///
///  - S. Vigna, "Broadword implementation of rank/select queries," In WEA, 2008.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rank9Sel {
    bv: RawBitVector,
    rs: Rank9SelIndex,
}

impl Rank9Sel {
    /// Creates a new vector from input bit vector `bv`.
    pub fn new(bv: RawBitVector) -> Self {
        let rs = Rank9SelIndex::from_raw(&bv);
        Self { bv, rs }
    }

    /// Creates a new vector from input bit stream `bits`.
    ///
    /// # Arguments
    ///
    /// - `bits`: Bit stream.
    pub fn from_bits<I>(bits: I) -> Self
    where
        I: IntoIterator<Item = bool>,
    {
        Self::new(RawBitVector::from_bits(bits))
    }

    /// Returns the reference of the internal bit vector.
    pub const fn bit_vector(&self) -> &RawBitVector {
        &self.bv
    }

    /// Returns the reference of the internal rank/select index.
    pub const fn rs_index(&self) -> &Rank9SelIndex {
        &self.rs
    }

    /// Returns the number of bits stored.
    pub const fn len(&self) -> usize {
        self.bv.len()
    }

    /// Checks if the vector is empty.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Build for Rank9Sel {
    /// Creates a new vector from input bit stream `bits`.
    ///
    /// # Arguments
    ///
    /// - `bits`: Bit stream.
    /// - `with_rank`: Dummy.
    /// - `with_select1`: Flag to enable select1 hints.
    /// - `with_select0`: Flag to enable select0 hints.
    ///
    /// # Errors
    ///
    /// Never.
    fn build_from_bits<I>(
        bits: I,
        _with_rank: bool,
        with_select1: bool,
        with_select0: bool,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = bool>,
        Self: Sized,
    {
        let bv = RawBitVector::from_bits(bits);
        let mut builder = Rank9SelIndexBuilder::from_raw(&bv);
        if with_select1 {
            builder = builder.select1_hints();
        }
        if with_select0 {
            builder = builder.select0_hints();
        }
        let rs = builder.build();
        Ok(Self { bv, rs })
    }
}

impl NumBits for Rank9Sel {
    /// Returns the number of bits stored (just wrapping [`Self::len()`]).
    #[inline(always)]
    fn num_bits(&self) -> usize {
        self.len()
    }

    /// Returns the number of bits set.
    #[inline(always)]
    fn num_ones(&self) -> usize {
        self.rs.num_ones()
    }
}

impl Access for Rank9Sel {
    /// Returns the `pos`-th bit, or [`None`] if out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use jerky::bit_vectors::{Rank9Sel, Access};
    ///
    /// let bv = Rank9Sel::from_bits([true, false, false]);
    ///
    /// assert_eq!(bv.access(0), Some(true));
    /// assert_eq!(bv.access(1), Some(false));
    /// assert_eq!(bv.access(2), Some(false));
    /// assert_eq!(bv.access(3), None);
    /// ```
    fn access(&self, pos: usize) -> Option<bool> {
        self.bv.access(pos)
    }
}

impl Rank for Rank9Sel {
    /// Returns the number of ones from the 0-th bit to the `pos-1`-th bit, or
    /// [`None`] if `self.len() < pos`.
    ///
    /// # Complexity
    ///
    /// Constant
    ///
    /// # Examples
    ///
    /// ```
    /// use jerky::bit_vectors::{Rank9Sel, Rank};
    ///
    /// let bv = Rank9Sel::from_bits([true, false, false, true]);
    ///
    /// assert_eq!(bv.rank1(1), Some(1));
    /// assert_eq!(bv.rank1(2), Some(1));
    /// assert_eq!(bv.rank1(3), Some(1));
    /// assert_eq!(bv.rank1(4), Some(2));
    /// assert_eq!(bv.rank1(5), None);
    /// ```
    fn rank1(&self, pos: usize) -> Option<usize> {
        let data = BitVectorData::from(self.bv.clone());
        self.rs.rank1(&data, pos)
    }

    /// Returns the number of zeros from the 0-th bit to the `pos-1`-th bit, or
    /// [`None`] if `self.len() < pos`.
    ///
    /// # Complexity
    ///
    /// Constant
    ///
    /// # Examples
    ///
    /// ```
    /// use jerky::bit_vectors::{Rank9Sel, Rank};
    ///
    /// let bv = Rank9Sel::from_bits([true, false, false, true]);
    ///
    /// assert_eq!(bv.rank0(1), Some(0));
    /// assert_eq!(bv.rank0(2), Some(1));
    /// assert_eq!(bv.rank0(3), Some(2));
    /// assert_eq!(bv.rank0(4), Some(2));
    /// assert_eq!(bv.rank0(5), None);
    /// ```
    fn rank0(&self, pos: usize) -> Option<usize> {
        let data = BitVectorData::from(self.bv.clone());
        self.rs.rank0(&data, pos)
    }
}

impl Select for Rank9Sel {
    /// Searches the position of the `k`-th bit set, or
    /// [`None`] if `self.num_ones() <= k`.
    ///
    /// # Complexity
    ///
    /// Logarithmic
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use jerky::bit_vectors::{Rank9Sel, Select, Build};
    ///
    /// let bv = Rank9Sel::build_from_bits([true, false, false, true], false, true, false)?;
    ///
    /// assert_eq!(bv.select1(0), Some(0));
    /// assert_eq!(bv.select1(1), Some(3));
    /// assert_eq!(bv.select1(2), None);
    /// # Ok(())
    /// # }
    /// ```
    fn select1(&self, k: usize) -> Option<usize> {
        let data = BitVectorData::from(self.bv.clone());
        self.rs.select1(&data, k)
    }

    /// Searches the position of the `k`-th bit unset, or
    /// [`None`] if `self.num_zeros() <= k`.
    ///
    /// # Complexity
    ///
    /// Logarithmic
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use jerky::bit_vectors::{Rank9Sel, Select, Build};
    ///
    /// let bv = Rank9Sel::build_from_bits([true, false, false, true], false, false, true)?;
    ///
    /// assert_eq!(bv.select0(0), Some(1));
    /// assert_eq!(bv.select0(1), Some(2));
    /// assert_eq!(bv.select0(2), None);
    /// # Ok(())
    /// # }
    /// ```
    fn select0(&self, k: usize) -> Option<usize> {
        let data = BitVectorData::from(self.bv.clone());
        self.rs.select0(&data, k)
    }
}

impl Rank9Sel {
    /// Returns the number of bytes required for the old copy-based serialization.
    pub fn size_in_bytes(&self) -> usize {
        self.bv.size_in_bytes() + self.rs.size_in_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rank1_all_zeros() {
        let bv = Rank9Sel::from_bits([false, false, false]);
        assert_eq!(bv.rank1(0), Some(0));
        assert_eq!(bv.rank1(1), Some(0));
        assert_eq!(bv.rank1(2), Some(0));
        assert_eq!(bv.rank1(3), Some(0));
        assert_eq!(bv.rank1(4), None);
    }

    #[test]
    fn test_select1_all_zeros() {
        let bv = Rank9Sel::build_from_bits([false, false, false], false, true, false).unwrap();
        assert_eq!(bv.select1(0), None);
    }

    #[test]
    fn test_rank0_all_ones() {
        let bv = Rank9Sel::from_bits([true, true, true]);
        assert_eq!(bv.rank0(0), Some(0));
        assert_eq!(bv.rank0(1), Some(0));
        assert_eq!(bv.rank0(2), Some(0));
        assert_eq!(bv.rank0(3), Some(0));
        assert_eq!(bv.rank0(4), None);
    }

    #[test]
    fn test_select0_all_ones() {
        let bv = Rank9Sel::build_from_bits([true, true, true], false, false, true).unwrap();
        assert_eq!(bv.select0(0), None);
    }

    #[test]
    fn test_select0_no_hint() {
        let bv = Rank9Sel::from_bits([true, false, false, true]);
        assert_eq!(bv.select0(0), Some(1));
        assert_eq!(bv.select0(1), Some(2));
        assert_eq!(bv.select0(2), None);
    }

    #[test]
    fn test_select1_no_hint() {
        let bv = Rank9Sel::from_bits([true, false, false, true]);
        assert_eq!(bv.select1(0), Some(0));
        assert_eq!(bv.select1(1), Some(3));
        assert_eq!(bv.select1(2), None);
    }
}
