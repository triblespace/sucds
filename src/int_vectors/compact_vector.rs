//! Updatable compact vector in which each integer is represented in a fixed number of bits.
#![cfg(target_pointer_width = "64")]

use anyhow::{anyhow, Result};
use num_traits::ToPrimitive;

use crate::bit_vector::BitVectorBuilder;
use crate::bit_vector::{BitVector, BitVectorData, NoIndex};
use crate::int_vectors::prelude::*;
use crate::utils;
use anybytes::Bytes;

/// Mutable builder for [`CompactVector`].
///
/// This structure collects integers using [`push_int`], [`set_int`], or
/// [`extend`] and finally [`freeze`]s into an immutable [`CompactVector`].
///
/// # Examples
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use jerky::int_vectors::CompactVectorBuilder;
/// let mut builder = CompactVectorBuilder::new(3)?;
/// builder.push_int(1)?;
/// builder.extend([2, 5])?;
/// let cv = builder.freeze();
/// assert_eq!(cv.get_int(1), Some(2));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default, Clone)]
pub struct CompactVectorBuilder {
    chunks: BitVectorBuilder,
    len: usize,
    width: usize,
}

impl CompactVectorBuilder {
    /// Creates a new empty builder storing integers within `width` bits each.
    ///
    /// # Arguments
    ///
    /// * `width` - Number of bits used to store each integer.
    ///
    /// # Errors
    ///
    /// Returns an error if `width` is outside `1..=64`.
    pub fn new(width: usize) -> Result<Self> {
        if !(1..=64).contains(&width) {
            return Err(anyhow!("width must be in 1..=64, but got {width}."));
        }
        Ok(Self {
            chunks: BitVectorBuilder::new(),
            len: 0,
            width,
        })
    }

    /// Creates a new builder reserving space for at least `capa` integers.
    ///
    /// Currently the reservation is ignored as the builder grows
    /// automatically.
    pub fn with_capacity(_capa: usize, width: usize) -> Result<Self> {
        Self::new(width)
    }

    /// Pushes integer `val` at the end.
    ///
    /// # Errors
    ///
    /// Returns an error if `val` cannot be represented in `self.width()` bits.
    pub fn push_int(&mut self, val: usize) -> Result<()> {
        if self.width != 64 && val >> self.width != 0 {
            return Err(anyhow!(
                "val must fit in self.width()={} bits, but got {val}.",
                self.width
            ));
        }
        self.chunks.push_bits(val, self.width)?;
        self.len += 1;
        Ok(())
    }

    /// Sets the `pos`-th integer to `val`.
    ///
    /// # Errors
    ///
    /// Returns an error if `pos` is out of bounds or if `val` does not fit in
    /// `self.width()` bits.
    pub fn set_int(&mut self, pos: usize, val: usize) -> Result<()> {
        if self.len <= pos {
            return Err(anyhow!(
                "pos must be no greater than self.len()={}, but got {pos}.",
                self.len
            ));
        }
        if self.width != 64 && val >> self.width != 0 {
            return Err(anyhow!(
                "val must fit in self.width()={} bits, but got {val}.",
                self.width
            ));
        }
        for i in 0..self.width {
            let bit = ((val >> i) & 1) == 1;
            self.chunks.set_bit(pos * self.width + i, bit)?;
        }
        Ok(())
    }

    /// Appends integers at the end.
    ///
    /// # Errors
    ///
    /// Returns an error if any value does not fit in `self.width()` bits.
    pub fn extend<I>(&mut self, vals: I) -> Result<()>
    where
        I: IntoIterator<Item = usize>,
    {
        for x in vals {
            self.push_int(x)?;
        }
        Ok(())
    }

    /// Finalizes the builder into an immutable [`CompactVector`].
    ///
    /// The builder can no longer be used after freezing.
    ///
    /// # Examples
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use jerky::int_vectors::CompactVectorBuilder;
    /// let cv = CompactVectorBuilder::new(2)?.freeze();
    /// assert!(cv.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub fn freeze(self) -> CompactVector {
        let chunks: BitVector<NoIndex> = self.chunks.freeze::<NoIndex>();
        CompactVector {
            chunks,
            len: self.len,
            width: self.width,
        }
    }
}

/// Updatable compact vector in which each integer is represented in a fixed number of bits.
///
/// # Memory usage
///
/// $`n \lceil \lg u \rceil`$ bits for $`n`$ integers in which a value is in $`[0,u)`$.
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use jerky::int_vectors::{CompactVector, CompactVectorBuilder};
///
/// // Can store integers within 3 bits each.
/// let mut builder = CompactVectorBuilder::new(3)?;
/// builder.push_int(7)?;
/// builder.push_int(2)?;
/// builder.set_int(0, 5)?;
/// let cv = builder.freeze();
///
/// assert_eq!(cv.len(), 2);
/// assert_eq!(cv.get_int(0), Some(5));
/// # Ok(())
/// # }
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct CompactVector {
    chunks: BitVector<NoIndex>,
    len: usize,
    width: usize,
}

impl Default for CompactVector {
    fn default() -> Self {
        Self {
            chunks: BitVectorBuilder::new().freeze::<NoIndex>(),
            len: 0,
            width: 0,
        }
    }
}

/// Metadata returned by [`CompactVector::to_bytes`] and required by
/// [`CompactVector::from_bytes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactVectorMeta {
    /// Number of integers stored.
    pub len: usize,
    /// Bit width for each integer.
    pub width: usize,
}

impl CompactVector {
    /// Creates a new empty builder storing integers within `width` bits each.
    ///
    /// # Arguments
    ///
    ///  - `width`: Number of bits used to store an integer.
    ///
    /// # Errors
    ///
    /// An error is returned if `width` is not in `1..=64`.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use jerky::int_vectors::CompactVector;
    ///
    /// let cv = CompactVector::new(3)?.freeze();
    /// assert_eq!(cv.len(), 0);
    /// assert_eq!(cv.width(), 3);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(width: usize) -> Result<CompactVectorBuilder> {
        CompactVectorBuilder::new(width)
    }

    /// Creates a new builder storing integers in `width` bits and
    /// reserving space for at least `capa` integers.
    ///
    /// # Arguments
    ///
    ///  - `capa`: Number of elements reserved at least.
    ///  - `width`: Number of bits used to store an integer.
    ///
    /// # Errors
    ///
    /// An error is returned if `width` is not in `1..=64`.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use jerky::int_vectors::CompactVector;
    ///
    /// let cv = CompactVector::with_capacity(10, 3)?.freeze();
    ///
    /// assert_eq!(cv.len(), 0);
    /// assert_eq!(cv.width(), 3);
    /// assert_eq!(cv.capacity(), 0);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_capacity(capa: usize, width: usize) -> Result<CompactVectorBuilder> {
        CompactVectorBuilder::with_capacity(capa, width)
    }

    /// Creates a new vector storing an integer in `width` bits,
    /// which stores `len` values initialized by `val`.
    ///
    /// # Arguments
    ///
    ///  - `val`: Integer value.
    ///  - `len`: Number of elements.
    ///  - `width`: Number of bits used to store an integer.
    ///
    /// # Errors
    ///
    /// An error is returned if
    ///
    ///  - `width` is not in `1..=64`, or
    ///  - `val` cannot be represent in `width` bits.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use jerky::int_vectors::CompactVector;
    ///
    /// let mut cv = CompactVector::from_int(7, 2, 3)?;
    /// assert_eq!(cv.len(), 2);
    /// assert_eq!(cv.width(), 3);
    /// assert_eq!(cv.get_int(0), Some(7));
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_int(val: usize, len: usize, width: usize) -> Result<Self> {
        if !(1..=64).contains(&width) {
            return Err(anyhow!("width must be in 1..=64, but got {width}."));
        }
        if width < 64 && val >> width != 0 {
            return Err(anyhow!(
                "val must fit in width={width} bits, but got {val}."
            ));
        }
        let mut builder = CompactVectorBuilder::with_capacity(len, width)?;
        for _ in 0..len {
            builder.push_int(val)?;
        }
        Ok(builder.freeze())
    }

    /// Creates a new vector from a slice of integers `vals`.
    ///
    /// The width of each element automatically fits to the maximum value in `vals`.
    ///
    /// # Arguments
    ///
    ///  - `vals`: Slice of integers to be stored.
    ///
    /// # Errors
    ///
    /// An error is returned if `vals` contains an integer that cannot be cast to [`usize`].
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use jerky::int_vectors::CompactVector;
    ///
    /// let mut cv = CompactVector::from_slice(&[7, 2])?;
    /// assert_eq!(cv.len(), 2);
    /// assert_eq!(cv.width(), 3);
    /// assert_eq!(cv.get_int(0), Some(7));
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_slice<T>(vals: &[T]) -> Result<Self>
    where
        T: ToPrimitive,
    {
        if vals.is_empty() {
            return Ok(Self::default());
        }
        let mut max_int = 0;
        for x in vals {
            max_int =
                max_int.max(x.to_usize().ok_or_else(|| {
                    anyhow!("vals must consist only of values castable into usize.")
                })?);
        }
        let mut builder =
            CompactVectorBuilder::with_capacity(vals.len(), utils::needed_bits(max_int))?;
        for x in vals {
            builder.push_int(x.to_usize().unwrap())?;
        }
        Ok(builder.freeze())
    }

    /// Returns the `pos`-th integer, or [`None`] if out of bounds.
    ///
    /// # Arguments
    ///
    ///  - `pos`: Position.
    ///
    /// # Complexity
    ///
    /// Constant
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use jerky::int_vectors::CompactVector;
    ///
    /// let cv = CompactVector::from_slice(&[5, 256, 0])?;
    /// assert_eq!(cv.get_int(0), Some(5));
    /// assert_eq!(cv.get_int(1), Some(256));
    /// assert_eq!(cv.get_int(2), Some(0));
    /// assert_eq!(cv.get_int(3), None);
    /// # Ok(())
    /// # }
    pub fn get_int(&self, pos: usize) -> Option<usize> {
        self.chunks.get_bits(pos * self.width, self.width)
    }

    /// Sets the `pos`-th integer to `val`.

    /// Creates an iterator for enumerating integers.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use jerky::int_vectors::CompactVector;
    ///
    /// let cv = CompactVector::from_slice(&[5, 256, 0])?;
    /// let mut it = cv.iter();
    ///
    /// assert_eq!(it.next(), Some(5));
    /// assert_eq!(it.next(), Some(256));
    /// assert_eq!(it.next(), Some(0));
    /// assert_eq!(it.next(), None);
    /// # Ok(())
    /// # }
    /// ```
    pub const fn iter(&self) -> Iter {
        Iter::new(self)
    }

    /// Collects all integers into a `Vec<usize>` for inspection.
    pub fn to_vec(&self) -> Vec<usize> {
        self.iter().collect()
    }

    /// Gets the number of integers.
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Checks if the vector is empty.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the total number of integers it can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.len()
    }

    /// Gets the number of bits to represent an integer.
    #[inline(always)]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Serializes the vector into a [`Bytes`] buffer and accompanying metadata.
    pub fn to_bytes(&self) -> (CompactVectorMeta, Bytes) {
        let (_, bytes) = self.chunks.data.to_bytes();
        (
            CompactVectorMeta {
                len: self.len,
                width: self.width,
            },
            bytes,
        )
    }

    /// Reconstructs the vector from zero-copy [`Bytes`] and its metadata.
    pub fn from_bytes(meta: CompactVectorMeta, bytes: Bytes) -> Result<Self> {
        let data_len = meta.len * meta.width;
        let data = BitVectorData::from_bytes(data_len, bytes)?;
        let chunks = BitVector::new(data, NoIndex);
        Ok(Self {
            chunks,
            len: meta.len,
            width: meta.width,
        })
    }
}

impl Build for CompactVector {
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

impl NumVals for CompactVector {
    /// Returns the number of integers stored (just wrapping [`Self::len()`]).
    fn num_vals(&self) -> usize {
        self.len()
    }
}

impl Access for CompactVector {
    /// Returns the `pos`-th integer, or [`None`] if out of bounds
    /// (just wrapping [`Self::get_int()`]).
    ///
    /// # Arguments
    ///
    ///  - `pos`: Position.
    ///
    /// # Complexity
    ///
    /// Constant
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use jerky::int_vectors::{CompactVector, Access};
    ///
    /// let cv = CompactVector::from_slice(&[5, 256, 0])?;
    /// assert_eq!(cv.access(0), Some(5));
    /// assert_eq!(cv.access(1), Some(256));
    /// assert_eq!(cv.access(2), Some(0));
    /// assert_eq!(cv.access(3), None);
    /// # Ok(())
    /// # }
    fn access(&self, pos: usize) -> Option<usize> {
        self.get_int(pos)
    }
}

/// Iterator for enumerating integers, created by [`CompactVector::iter()`].
pub struct Iter<'a> {
    cv: &'a CompactVector,
    pos: usize,
}

impl<'a> Iter<'a> {
    /// Creates a new iterator.
    pub const fn new(cv: &'a CompactVector) -> Self {
        Self { cv, pos: 0 }
    }
}

impl Iterator for Iter<'_> {
    type Item = usize;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.cv.len() {
            let x = self.cv.access(self.pos).unwrap();
            self.pos += 1;
            Some(x)
        } else {
            None
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.cv.len(), Some(self.cv.len()))
    }
}

impl std::fmt::Debug for CompactVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ints = vec![0; self.len()];
        for (i, b) in ints.iter_mut().enumerate() {
            *b = self.access(i).unwrap();
        }
        f.debug_struct("CompactVector")
            .field("ints", &ints)
            .field("len", &self.len)
            .field("width", &self.width)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_oob_0() {
        let e = CompactVector::new(0);
        assert_eq!(
            e.err().map(|x| x.to_string()),
            Some("width must be in 1..=64, but got 0.".to_string())
        );
    }

    #[test]
    fn test_new_oob_65() {
        let e = CompactVector::new(65);
        assert_eq!(
            e.err().map(|x| x.to_string()),
            Some("width must be in 1..=64, but got 65.".to_string())
        );
    }

    #[test]
    fn test_with_capacity_oob_0() {
        let e = CompactVector::with_capacity(0, 0);
        assert_eq!(
            e.err().map(|x| x.to_string()),
            Some("width must be in 1..=64, but got 0.".to_string())
        );
    }

    #[test]
    fn test_with_capacity_oob_65() {
        let e = CompactVector::with_capacity(0, 65);
        assert_eq!(
            e.err().map(|x| x.to_string()),
            Some("width must be in 1..=64, but got 65.".to_string())
        );
    }

    #[test]
    fn test_from_int_oob_0() {
        let e = CompactVector::from_int(0, 0, 0);
        assert_eq!(
            e.err().map(|x| x.to_string()),
            Some("width must be in 1..=64, but got 0.".to_string())
        );
    }

    #[test]
    fn test_from_int_oob_65() {
        let e = CompactVector::from_int(0, 0, 65);
        assert_eq!(
            e.err().map(|x| x.to_string()),
            Some("width must be in 1..=64, but got 65.".to_string())
        );
    }

    #[test]
    fn test_from_int_unfit() {
        let e = CompactVector::from_int(4, 0, 2);
        assert_eq!(
            e.err().map(|x| x.to_string()),
            Some("val must fit in width=2 bits, but got 4.".to_string())
        );
    }

    #[test]
    fn test_from_slice_uncastable() {
        let e = CompactVector::from_slice(&[u128::MAX]);
        assert_eq!(
            e.err().map(|x| x.to_string()),
            Some("vals must consist only of values castable into usize.".to_string())
        );
    }

    #[test]
    fn test_set_int_oob() {
        let mut builder = CompactVectorBuilder::with_capacity(1, 2).unwrap();
        builder.push_int(0).unwrap();
        let e = builder.set_int(1, 1);
        assert_eq!(
            e.err().map(|x| x.to_string()),
            Some("pos must be no greater than self.len()=1, but got 1.".to_string())
        );
    }

    #[test]
    fn test_set_int_unfit() {
        let mut builder = CompactVectorBuilder::with_capacity(1, 2).unwrap();
        builder.push_int(0).unwrap();
        let e = builder.set_int(0, 4);
        assert_eq!(
            e.err().map(|x| x.to_string()),
            Some("val must fit in self.width()=2 bits, but got 4.".to_string())
        );
    }

    #[test]
    fn test_push_int_unfit() {
        let mut builder = CompactVectorBuilder::new(2).unwrap();
        let e = builder.push_int(4);
        assert_eq!(
            e.err().map(|x| x.to_string()),
            Some("val must fit in self.width()=2 bits, but got 4.".to_string())
        );
    }

    #[test]
    fn test_extend_unfit() {
        let mut builder = CompactVectorBuilder::new(2).unwrap();
        let e = builder.extend([4]);
        assert_eq!(
            e.err().map(|x| x.to_string()),
            Some("val must fit in self.width()=2 bits, but got 4.".to_string())
        );
    }

    #[test]
    fn test_64b() {
        let mut builder = CompactVectorBuilder::new(64).unwrap();
        builder.push_int(42).unwrap();
        assert_eq!(builder.clone().freeze().get_int(0), Some(42));
        builder.set_int(0, 334).unwrap();
        let cv = builder.freeze();
        assert_eq!(cv.get_int(0), Some(334));
    }

    #[test]
    fn test_64b_from_int() {
        let cv = CompactVector::from_int(42, 1, 64).unwrap();
        assert_eq!(cv.get_int(0), Some(42));
    }

    #[test]
    fn iter_collects() {
        let cv = CompactVector::from_slice(&[1, 2, 3]).unwrap();
        let collected: Vec<usize> = cv.iter().collect();
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[test]
    fn to_vec_collects() {
        let cv = CompactVector::from_slice(&[1, 2, 3]).unwrap();
        assert_eq!(cv.to_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn from_bytes_roundtrip() {
        let cv = CompactVector::from_slice(&[4, 5, 6]).unwrap();
        let (meta, bytes) = cv.to_bytes();
        let other = CompactVector::from_bytes(meta, bytes).unwrap();
        assert_eq!(cv, other);
    }
}
