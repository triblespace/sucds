//! Top module for bit vectors.
//!
//! # Introduction
//!
//! Bit vectors and operations on them are fundamental to succinct data structures.
//!
//! Let $`S \subseteq \{ 0,1,\dots,u-1 \}`$ be a set of positions
//! at which bits are set in a bit vector of length $`u`$.
//! Our bit vectors support the following queries:
//!
//! - $`\textrm{Access}(i)`$ returns `true` if $`i \in S`$ or `false` otherwise (implemented by [`Access`]).
//! - $`\textrm{Rank}(i)`$ returns the cardinality of $`\{ x \in S \mid x < i \}`$ (implemented by [`Rank`]).
//! - $`\textrm{Select}(k)`$ returns the $`k`$-th smallest position in $`S`$ (implemented by [`Select`]).
//! - $`\textrm{Update}(i)`$ inserts/removes $`i`$ to/from $`S`$.
//!
//! Note that they are not limited depending on data structures.
//!
//! # Data structures
//!
//! Let $`n`$ be the number of positions (i.e., $`n = |S|`$).
//! The implementations provided in this crate are summarized below:
//!
//! | Implementations | [Access](Access) | [Rank](Rank) | [Select](Select) | Update | Memory (bits) |
//! | --- | :-: | :-: | :-: | :-: | :-: |
//! | [`BitVector`] | $`O(1)`$  | $`O(u)`$ | $`O(u)`$ | $`O(1)`$ | $`u`$ |
//! | [`BitVector<rank9sel::inner::Rank9SelIndex>`] | $`O(1)`$ | $`O(1)`$ | $`O(\lg u)`$ | -- | $`u + o(u)`$ |
//! | [`BitVector<darray::inner::DArrayFullIndex>`] | $`O(1)`$ | $`O(1)`$ | $`O(1)`$ | -- | $`u + o(u)`$ |
//!
//! ## Plain bit vectors without index
//!
//! [`BitVector`] is a plain format without index and the only mutable data structure.
//!
//! All search queries are performed by linear scan in $`O(u)`$ time,
//! although they are quickly computed in word units using bit-parallelism techniques.
//!
//! ## Plain bit vectors with index
//!
//! [`BitVector<rank9sel::inner::Rank9SelIndex>`] and [`BitVector<darray::inner::DArrayFullIndex>`] are index structures for faster queries built on [`BitVector`].
//!
//! [`BitVector<rank9sel::inner::Rank9SelIndex>`] is an implementation of Vigna's Rank9 and hinted selection techniques, supporting
//! constant-time Rank and logarithmic-time Select queries.
//!
//! [`BitVector<darray::inner::DArrayFullIndex>`] implements the dense array technique of Okanohara and Sadakane.
//! If you need only Select queries on dense sets (i.e., $`n/u \approx 0.5`$), this will be the most candidate.
//! Rank/Predecessor/Successor queries are optionally enabled using the [`Rank9SelIndex`](rank9sel::inner::Rank9SelIndex) index.
//! This structure outperforms [`BitVector<rank9sel::inner::Rank9SelIndex>`] in complexity, but the practical space overhead can be larger.
//!
//!
//! # Examples
//!
//! This module provides several traits for essential behaviors,
//! allowing to compare our bit vectors as components in your data structures.
//! [`prelude`] allows you to import them easily.
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use jerky::bit_vectors::{rank9sel::inner::Rank9SelIndex, BitVector, prelude::*};
//!
//! let bv: BitVector<Rank9SelIndex> = BitVector::build_from_bits(
//!     [true, false, false, true],
//!     true, // Enables rank1/0
//!     true, // Enables select1
//!     true  // Enables select0
//! )?;
//!
//! assert_eq!(bv.num_bits(), 4);
//! assert_eq!(bv.num_ones(), 2);
//!
//! assert_eq!(bv.access(1), Some(false));
//!
//! assert_eq!(bv.rank1(1), Some(1));
//! assert_eq!(bv.rank0(1), Some(0));
//!
//! assert_eq!(bv.select1(1), Some(3));
//! assert_eq!(bv.select0(0), Some(1));
//! # Ok(())
//! # }
//! ```
pub mod bit_vector;
pub mod darray;
pub mod data;
pub mod prelude;
pub mod rank9sel;

pub use data::{BitVector, BitVectorData, BitVectorIndex, IndexBuilder, NoIndex};

use anyhow::Result;

/// Interface for building a bit vector with rank/select queries.
pub trait Build {
    /// Creates a new vector from input bit stream `bits`.
    ///
    /// A data structure may not support a part of rank/select queries in the default
    /// configuration. The last three flags allow to enable them if optionally supported.
    ///
    /// # Arguments
    ///
    /// - `bits`: Bit stream.
    /// - `with_rank`: Flag to enable rank1/0.
    /// - `with_select1`: Flag to enable select1.
    /// - `with_select0`: Flag to enable select0.
    ///
    /// # Errors
    ///
    /// An error is returned if specified queries are not supported.
    fn build_from_bits<I>(
        bits: I,
        with_rank: bool,
        with_select1: bool,
        with_select0: bool,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = bool>,
        Self: Sized;
}

/// Interface for reporting basic statistics in a bit vector.
pub trait NumBits {
    /// Returns the number of bits stored.
    fn num_bits(&self) -> usize;

    /// Returns the number of bits set.
    fn num_ones(&self) -> usize;

    /// Returns the number of bits unset.
    #[inline(always)]
    fn num_zeros(&self) -> usize {
        self.num_bits() - self.num_ones()
    }
}

/// Interface for accessing elements on bit arrays.
pub trait Access {
    /// Returns the `pos`-th bit, or [`None`] if out of bounds.
    fn access(&self, pos: usize) -> Option<bool>;
}

/// Interface for rank queries on bit vectors.
///
/// Let $`S \subseteq \{ 0,1,\dots,u-1 \}`$ be a set of positions
/// at which bits are set in a bit vector of length $`u`$.
pub trait Rank {
    /// Returns the cardinality of $`\{ x \in S \mid x < i \}`$,
    /// or [`None`] if $`u < x`$.
    fn rank1(&self, x: usize) -> Option<usize>;

    /// Returns the cardinality of $`\{ x \not\in S \mid 0 \leq x < i \}`$,
    /// or [`None`] if $`u < x`$.
    fn rank0(&self, x: usize) -> Option<usize>;
}

/// Interface for select queries on bit vectors.
///
/// Let $`S \subseteq \{ 0,1,\dots,u-1 \}`$ be a set of positions
/// at which bits are set in a bit vector of length $`u`$.
pub trait Select {
    /// Returns the $`k`$-th smallest position in $`S`$, or
    /// [`None`] if out of bounds.
    fn select1(&self, k: usize) -> Option<usize>;

    /// Returns the $`k`$-th smallest integer $`x`$ such that $`x \not\in S`$ and $`0 \leq x < u`$, or
    /// [`None`] if out of bounds.
    fn select0(&self, k: usize) -> Option<usize>;
}
