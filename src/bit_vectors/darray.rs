//! Constant-time select data structure over integer sets with the dense array technique.
#![cfg(target_pointer_width = "64")]

pub mod inner;

use anyhow::Result;

use crate::bit_vectors::data::BitVectorData;
use crate::bit_vectors::prelude::*;
use crate::bit_vectors::rank9sel::inner::Rank9SelIndex;
use crate::bit_vectors::RawBitVector;
use inner::{DArrayIndex, DArrayIndexBuilder};

/// Constant-time select data structure over integer sets with the dense array technique.
///
/// # Memory complexity
///
/// $`u + o(u)`$ bits for a bit vector with $`u`$ bits.
///
/// # Notes
///
/// In the default configuration, this data structure supports only [`Self::select1()`].
/// If rank queries are needed, [`Self::enable_rank()`] and [`Self::enable_select0()`] must be set up.
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use jerky::bit_vectors::{DArray, Access, Rank, Select};
///
/// let da = DArray::from_bits([true, false, false, true])
///     .enable_rank()
///     .enable_select0();
///
/// assert_eq!(da.len(), 4);
/// assert_eq!(da.access(1), Some(false));
///
/// assert_eq!(da.rank1(1), Some(1));
/// assert_eq!(da.rank0(1), Some(0));
///
/// assert_eq!(da.select1(1), Some(3));
/// assert_eq!(da.select0(0), Some(1));
/// # Ok(())
/// # }
/// ```
///
/// # Credits
///
/// This is a yet another Rust port of [succinct::darray](https://github.com/ot/succinct/blob/master/darray.hpp).
///
/// # References
///
///  - D. Okanohara, and K. Sadakane, "Practical Entropy-Compressed Rank/Select Dictionary,"
///    In ALENEX, 2007.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct DArray {
    bv: RawBitVector,
    s1: DArrayIndex<true>,
    s0: Option<DArrayIndex<false>>,
    r9: Option<Rank9SelIndex>,
}

impl DArray {
    /// Creates a new instance from input bit stream `bits`.
    ///
    /// # Arguments
    ///
    /// - `bits`: Bit stream.
    pub fn from_bits<I>(bits: I) -> Self
    where
        I: IntoIterator<Item = bool>,
    {
        let bv = RawBitVector::from_bits(bits);
        let s1 = DArrayIndexBuilder::<true>::from_raw(&bv).build();
        Self {
            bv,
            s1,
            s0: None,
            r9: None,
        }
    }

    /// Builds an index to enable rank queries.
    #[must_use]
    pub fn enable_rank(mut self) -> Self {
        self.r9 = Some(Rank9SelIndex::from_raw(&self.bv));
        self
    }

    /// Builds an index to enable select0.
    #[must_use]
    pub fn enable_select0(mut self) -> Self {
        self.s0 = Some(DArrayIndexBuilder::<false>::from_raw(&self.bv).build());
        self
    }

    /// Checks if [`Self::enable_rank()`] is set.
    #[inline(always)]
    pub const fn has_rank(&self) -> bool {
        self.r9.is_some()
    }

    /// Checks if [`Self::enable_select0()`] is set.
    #[inline(always)]
    pub const fn has_select0(&self) -> bool {
        self.s0.is_some()
    }

    /// Returns the reference of the internal bit vector.
    pub const fn bit_vector(&self) -> &RawBitVector {
        &self.bv
    }

    /// Returns the reference of the internal select1 index.
    pub const fn s1_index(&self) -> &DArrayIndex<true> {
        &self.s1
    }

    /// Returns the reference of the internal select0 index.
    pub const fn s0_index(&self) -> Option<&DArrayIndex<false>> {
        self.s0.as_ref()
    }

    /// Returns the reference of the internal rank index.
    pub const fn r9_index(&self) -> Option<&Rank9SelIndex> {
        self.r9.as_ref()
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

impl Build for DArray {
    /// Creates a new vector from input bit stream `bits`.
    ///
    /// # Arguments
    ///
    /// - `bits`: Bit stream.
    /// - `with_rank`: Flag to enable [`Self::enable_rank()`].
    /// - `with_select1`: Dummy.
    /// - `with_select0`: Flag to enable [`Self::enable_select0()`].
    ///
    /// # Errors
    ///
    /// Never.
    fn build_from_bits<I>(
        bits: I,
        with_rank: bool,
        _with_select1: bool,
        with_select0: bool,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = bool>,
        Self: Sized,
    {
        let mut rsbv = Self::from_bits(bits);
        if with_rank {
            rsbv = rsbv.enable_rank();
        }
        if with_select0 {
            rsbv = rsbv.enable_select0();
        }
        Ok(rsbv)
    }
}

impl NumBits for DArray {
    /// Returns the number of bits stored (just wrapping [`Self::len()`]).
    #[inline(always)]
    fn num_bits(&self) -> usize {
        self.len()
    }

    /// Returns the number of bits set.
    #[inline(always)]
    fn num_ones(&self) -> usize {
        self.s1.num_ones()
    }
}

impl Access for DArray {
    /// Returns the `pos`-th bit, or [`None`] if out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use jerky::bit_vectors::{DArray, Access};
    ///
    /// let da = DArray::from_bits([true, false, false]);
    ///
    /// assert_eq!(da.access(0), Some(true));
    /// assert_eq!(da.access(1), Some(false));
    /// assert_eq!(da.access(2), Some(false));
    /// assert_eq!(da.access(3), None);
    /// ```
    fn access(&self, pos: usize) -> Option<bool> {
        self.bv.access(pos)
    }
}

impl Rank for DArray {
    /// Returns the number of ones from the 0-th bit to the `pos-1`-th bit, or
    /// [`None`] if `self.len() < pos`.
    ///
    /// # Complexity
    ///
    /// Constant
    ///
    /// # Panics
    ///
    /// It panics if the index is not built by [`Self::enable_rank()`].
    ///
    /// # Examples
    ///
    /// ```
    /// use jerky::bit_vectors::{DArray, Rank};
    ///
    /// let da = DArray::from_bits([true, false, false, true]).enable_rank();
    ///
    /// assert_eq!(da.rank1(1), Some(1));
    /// assert_eq!(da.rank1(2), Some(1));
    /// assert_eq!(da.rank1(3), Some(1));
    /// assert_eq!(da.rank1(4), Some(2));
    /// assert_eq!(da.rank1(5), None);
    /// ```
    fn rank1(&self, pos: usize) -> Option<usize> {
        let r9 = self.r9.as_ref().expect("enable_rank() must be set up.");
        let data = BitVectorData::from(self.bv.clone());
        r9.rank1(&data, pos)
    }

    /// Returns the number of zeros from the 0-th bit to the `pos-1`-th bit, or
    /// [`None`] if `self.len() < pos`.
    ///
    /// # Complexity
    ///
    /// Constant
    ///
    /// # Panics
    ///
    /// It panics if the index is not built by [`Self::enable_rank()`].
    ///
    /// # Examples
    ///
    /// ```
    /// use jerky::bit_vectors::{DArray, Rank};
    ///
    /// let da = DArray::from_bits([true, false, false, true]).enable_rank();
    ///
    /// assert_eq!(da.rank0(1), Some(0));
    /// assert_eq!(da.rank0(2), Some(1));
    /// assert_eq!(da.rank0(3), Some(2));
    /// assert_eq!(da.rank0(4), Some(2));
    /// assert_eq!(da.rank0(5), None);
    /// ```
    fn rank0(&self, pos: usize) -> Option<usize> {
        let r9 = self.r9.as_ref().expect("enable_rank() must be set up.");
        let data = BitVectorData::from(self.bv.clone());
        r9.rank0(&data, pos)
    }
}

impl Select for DArray {
    /// Searches the position of the `k`-th bit set, or
    /// [`None`] if `self.num_ones() <= k`.
    ///
    /// # Complexity
    ///
    /// Constant
    ///
    /// # Examples
    ///
    /// ```
    /// use jerky::bit_vectors::{DArray, Select};
    ///
    /// let da = DArray::from_bits([true, false, false, true]);
    ///
    /// assert_eq!(da.select1(0), Some(0));
    /// assert_eq!(da.select1(1), Some(3));
    /// assert_eq!(da.select1(2), None);
    /// ```
    fn select1(&self, k: usize) -> Option<usize> {
        let data = BitVectorData::from(self.bv.clone());
        self.s1.select(&data, k)
    }

    /// Searches the position of the `k`-th bit unset, or
    /// [`None`] if `self.num_zeros() <= k`.
    ///
    /// # Complexity
    ///
    /// Constant
    ///
    /// # Panics
    ///
    /// It panics if the index is not built by [`Self::enable_select0()`].
    ///
    /// # Examples
    ///
    /// ```
    /// use jerky::bit_vectors::{DArray, Select};
    ///
    /// let da = DArray::from_bits([true, false, false, true]).enable_select0();
    ///
    /// assert_eq!(da.select0(0), Some(1));
    /// assert_eq!(da.select0(1), Some(2));
    /// assert_eq!(da.select0(2), None);
    /// ```
    fn select0(&self, k: usize) -> Option<usize> {
        let s0 = self.s0.as_ref().expect("enable_select0() must be set up.");
        let data = BitVectorData::from(self.bv.clone());
        s0.select(&data, k)
    }
}

impl DArray {
    /// Returns the number of bytes required for the old copy-based serialization.
    pub fn size_in_bytes(&self) -> usize {
        self.bv.size_in_bytes()
            + self.s1.size_in_bytes()
            + self.s0.as_ref().map_or(std::mem::size_of::<bool>(), |x| {
                std::mem::size_of::<bool>() + x.size_in_bytes()
            })
            + std::mem::size_of::<bool>()
            + self.r9.as_ref().map_or(0, |r| r.size_in_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_zeros() {
        let da = DArray::from_bits([false, false, false]);
        assert_eq!(da.select1(0), None);
    }

    #[test]
    #[should_panic]
    fn test_rank1() {
        let da = DArray::from_bits([false, true, false]);
        da.rank1(1);
    }

    #[test]
    #[should_panic]
    fn test_rank0() {
        let da = DArray::from_bits([false, true, false]);
        da.rank0(1);
    }

    #[test]
    #[should_panic]
    fn test_select1() {
        let da = DArray::from_bits([false, true, false]);
        da.select0(0);
    }
}
