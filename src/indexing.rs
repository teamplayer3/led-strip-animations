use core::ops::{Deref, Range, RangeBounds};

use num::abs;

pub type Index = u16;

pub type LedId = Index;

#[derive(Debug)]
pub enum MappingError {
    NotInMappingRange,
    IndexOutOfBounds,
}

pub trait Indexing {
    type OutputIndex: ExactSizeIterator<Item = Index>;
    fn index(&self, index: Index) -> Result<Self::OutputIndex, MappingError>;
    fn len(&self) -> usize;
}

pub trait IndexingExt: Indexing {
    fn reversed(self) -> ReversedIndexed<Self>
    where
        Self: Sized,
    {
        ReversedIndexed::new(self)
    }

    fn split_into_half(
        self,
        uneven_behavior: UnevenBehavior,
    ) -> (HalfIndexed<Self>, HalfIndexed<Self>)
    where
        Self: Sized + Clone,
    {
        divided_indexing(self, uneven_behavior)
    }

    fn split_mirrored(self, uneven_behavior: UnevenBehavior) -> SplitMirroredIndexed<Self>
    where
        Self: Sized,
    {
        SplitMirroredIndexed::new(self, uneven_behavior)
    }

    fn every_nth(self, n: usize) -> EveryNthIndexed<Self>
    where
        Self: Sized,
    {
        EveryNthIndexed::new(self, n)
    }

    fn bounded(self, range: Range<LedId>) -> BoundedIndexed<Self>
    where
        Self: Sized,
    {
        BoundedIndexed::from_range(self, range)
    }

    fn circular(self, offset: isize) -> CircularIndexed<Self>
    where
        Self: Sized,
    {
        CircularIndexed::new(self, offset)
    }
}

impl<M: Indexing> IndexingExt for M {}

#[derive(Clone, Copy)]
pub struct ReversedIndexed<I>(I);

impl<I> ReversedIndexed<I> {
    pub fn new(indexer: I) -> Self {
        Self(indexer)
    }
}

impl<I: Indexing> Indexing for ReversedIndexed<I> {
    type OutputIndex = <I as Indexing>::OutputIndex;

    fn index(&self, index: Index) -> Result<Self::OutputIndex, MappingError> {
        self.0
            .index(Index::try_from(self.0.len()).unwrap() - index - 1)
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

pub fn reverse_indexing<I>(indexer: I) -> ReversedIndexed<I> {
    ReversedIndexed(indexer)
}

#[derive(Clone, Copy)]
pub struct EveryNthIndexed<I>(I, usize);

impl<I> EveryNthIndexed<I> {
    pub fn new(indexer: I, nth: usize) -> Self {
        Self(indexer, nth)
    }
}

impl<I: Indexing> Indexing for EveryNthIndexed<I> {
    type OutputIndex = <I as Indexing>::OutputIndex;

    fn index(&self, index: Index) -> Result<Self::OutputIndex, MappingError> {
        self.0.index(index * Index::try_from(self.1).unwrap())
    }

    fn len(&self) -> usize {
        self.0.len() / self.1
    }
}

pub fn every_nth_indexing<I>(indexer: I, nth: usize) -> EveryNthIndexed<I> {
    EveryNthIndexed(indexer, nth)
}

/// Will map the range to a circle which wraps around the bounds.
///
/// By the offset the start of the range can be shifted. If the index is out of bounds, it will return an error.
///
/// # Example
/// ```
/// # use led_strip_animations::indexing::{CircularIndexed, Indexing};
/// let indexes = [0, 1, 2, 3, 4, 5, 6, 7, 8];
/// let circle = CircularIndexed::new(&indexes, 2);
///
/// assert_eq!(*circle.index(0).unwrap(), 2);
/// assert_eq!(*circle.index(8).unwrap(), 1);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct CircularIndexed<I>(I, isize);

impl<I: Indexing> CircularIndexed<I> {
    pub fn new(indexer: I, offset: isize) -> Self {
        assert!(abs(offset) < indexer.len() as isize);
        Self(indexer, offset)
    }
}

impl<I: Indexing> Indexing for CircularIndexed<I> {
    type OutputIndex = <I as Indexing>::OutputIndex;

    fn index(&self, index: Index) -> Result<Self::OutputIndex, MappingError> {
        let len = Index::try_from(self.0.len()).unwrap();
        if index >= len {
            return Err(MappingError::IndexOutOfBounds);
        }

        let index_with_offset = (index as isize) + self.1;
        let index = if index_with_offset < 0 {
            len - Index::try_from(abs(index_with_offset)).unwrap()
        } else {
            Index::try_from((index as isize) + self.1).unwrap() % len
        };

        self.0.index(index)
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Clone, Copy)]
pub enum Bound {
    Relative(usize),
    Absolute(usize),
    None,
}

/// Will add bounds to the front and the end of a indexed range.
///
/// Bounds can be specified as absolute or relative. Absolute bounds will be counted from the start.
///
/// # Example
/// In this example we have an index range from 0 to 10. We want to map the range from 2 to 8.
/// ```
/// # use led_strip_animations::indexing::{BoundedIndexed, Bound};
/// let indexes = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
/// let range = BoundedIndexed::from_bounds(indexes, Bound::Absolute(2), Bound::Relative(2));
/// ```
#[derive(Clone, Copy)]
pub struct BoundedIndexed<I>(I, Bound, Bound);

impl<I: Indexing> BoundedIndexed<I> {
    /// Creates a new bounded index mapping.
    ///
    /// # Example
    /// ```
    /// # use led_strip_animations::indexing::{BoundedIndexed, Indexing};
    /// let indexes = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    /// let bounded = BoundedIndexed::from_range(&indexes, 2..7);
    ///
    /// assert_eq!(bounded.len(), 5);
    /// assert_eq!(*bounded.index(0).unwrap(), 2);
    /// assert_eq!(*bounded.index(4).unwrap(), 6);
    /// ```
    pub fn from_range<R: RangeBounds<LedId>>(indexer: I, range: R) -> Self {
        Self(
            indexer,
            core_bounds_to_bounds(range.start_bound(), true),
            core_bounds_to_bounds(range.end_bound(), false),
        )
    }

    /// Creates a new bounded index mapping.
    ///
    /// # Example
    /// ```
    /// # use led_strip_animations::indexing::{Bound, BoundedIndexed, Indexing};
    /// let indexes = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    /// let bounded = BoundedIndexed::from_bounds(&indexes, Bound::Absolute(2), Bound::Relative(2));
    ///
    /// assert_eq!(bounded.len(), 6);
    /// assert_eq!(*bounded.index(0).unwrap(), 2);
    /// assert_eq!(*bounded.index(5).unwrap(), 7);
    /// ```
    pub fn from_bounds(indexer: I, front_bound: Bound, end_bound: Bound) -> Self {
        Self(indexer, front_bound, end_bound)
    }

    fn front_off(&self) -> usize {
        match self.1 {
            Bound::None => 0,
            Bound::Relative(o) | Bound::Absolute(o) => o,
        }
    }

    fn end_off(&self) -> usize {
        match self.2 {
            Bound::None => 0,
            Bound::Relative(o) => o,
            Bound::Absolute(o) => self.0.len() - o - 1,
        }
    }
}

fn core_bounds_to_bounds(core_bound: core::ops::Bound<&LedId>, start: bool) -> Bound {
    let off = if start { 0 } else { 1 };
    match core_bound {
        core::ops::Bound::Included(o) => Bound::Absolute((*o + off) as usize),
        core::ops::Bound::Excluded(o) => Bound::Absolute((*o - off) as usize),
        core::ops::Bound::Unbounded => Bound::None,
    }
}

impl<I: Indexing> Indexing for BoundedIndexed<I> {
    type OutputIndex = <I as Indexing>::OutputIndex;

    fn index(&self, index: Index) -> Result<Self::OutputIndex, MappingError> {
        if index >= Index::try_from(self.len()).map_err(|_| MappingError::IndexOutOfBounds)? {
            return Err(MappingError::NotInMappingRange);
        }
        self.0
            .index(index + Index::try_from(self.front_off()).unwrap())
    }

    fn len(&self) -> usize {
        let front_off = self.front_off();
        let end_off = self.end_off();
        self.0.len() - front_off - end_off
    }
}

#[derive(Debug, Clone, Copy)]
pub enum UnevenBehavior {
    Exclude,
    ToLower,
    ToUpper,
}

/// Will split a index range in two and mirror the second half.
///
/// This is useful for animating a continuous range which is split in to two parts and the
/// animation should run on both parts mirrored.
#[derive(Debug, Clone, Copy)]
pub struct SplitMirroredIndexed<I>(I, UnevenBehavior);

impl<I> SplitMirroredIndexed<I> {
    pub fn new(indexer: I, uneven_behavior: UnevenBehavior) -> Self {
        Self(indexer, uneven_behavior)
    }
}

impl<I: Indexing<OutputIndex = SingleIndexed>> Indexing for SplitMirroredIndexed<I> {
    type OutputIndex = ManyIndexed<2>;

    fn index(&self, index: Index) -> Result<Self::OutputIndex, MappingError> {
        let own_len = Index::try_from(self.len()).unwrap();
        if index >= own_len {
            return Err(MappingError::NotInMappingRange);
        }

        let front_index = index;
        let back_index = Index::try_from(self.0.len()).unwrap() - index - 1;

        Ok(ManyIndexed::new([
            *self.0.index(front_index)?,
            *self.0.index(back_index)?,
        ]))
    }

    fn len(&self) -> usize {
        let indexed_len = self.0.len();
        if indexed_len % 2 != 0 {
            match self.1 {
                UnevenBehavior::Exclude => indexed_len / 2,
                UnevenBehavior::ToLower => (indexed_len + 1) / 2,
                UnevenBehavior::ToUpper => (indexed_len + 1) / 2,
            }
        } else {
            indexed_len / 2
        }
    }
}

#[derive(Clone, Copy)]
pub struct HalfIndexed<I>(I, bool, UnevenBehavior);

impl<I: Indexing> Indexing for HalfIndexed<I> {
    type OutputIndex = <I as Indexing>::OutputIndex;

    fn index(&self, index: Index) -> Result<Self::OutputIndex, MappingError> {
        if self.1 {
            self.0.index(index)
        } else {
            self.0
                .index(index + Index::try_from(self.0.len() - self.len()).unwrap())
        }
    }

    fn len(&self) -> usize {
        let inner_len = self.0.len();
        if inner_len % 2 != 0 {
            if self.1 {
                match self.2 {
                    UnevenBehavior::Exclude | UnevenBehavior::ToUpper => inner_len / 2,
                    UnevenBehavior::ToLower => inner_len / 2 + 1,
                }
            } else {
                match self.2 {
                    UnevenBehavior::Exclude | UnevenBehavior::ToLower => inner_len / 2,
                    UnevenBehavior::ToUpper => inner_len / 2 + 1,
                }
            }
        } else {
            inner_len / 2
        }
    }
}

pub fn divided_indexing<I: Clone>(
    indexer: I,
    uneven_behavior: UnevenBehavior,
) -> (HalfIndexed<I>, HalfIndexed<I>) {
    let lower_half = HalfIndexed(indexer.clone(), true, uneven_behavior);
    let upper_half = HalfIndexed(indexer, false, uneven_behavior);
    (lower_half, upper_half)
}

#[derive(Debug)]
pub struct ManyIndexed<const N: usize> {
    indexes: [LedId; N],
    index: usize,
}

impl<const N: usize> ManyIndexed<N> {
    pub fn new(indexes: [LedId; N]) -> Self {
        Self { indexes, index: 0 }
    }
}

impl<const N: usize> Iterator for ManyIndexed<N> {
    type Item = LedId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.indexes.len() > self.index {
            let index = self.index;
            self.index += 1;
            Some(self.indexes[index])
        } else {
            None
        }
    }
}

impl<const N: usize> ExactSizeIterator for ManyIndexed<N> {
    fn len(&self) -> usize {
        self.indexes.len()
    }
}

#[derive(Debug)]
pub struct SingleIndexed {
    index: LedId,
    called: bool,
}

impl SingleIndexed {
    pub fn new(index: LedId) -> Self {
        Self {
            index,
            called: false,
        }
    }
}

impl Deref for SingleIndexed {
    type Target = LedId;

    fn deref(&self) -> &Self::Target {
        &self.index
    }
}

impl Iterator for SingleIndexed {
    type Item = LedId;

    fn next(&mut self) -> Option<Self::Item> {
        match self.called {
            true => None,
            _ => {
                self.called = true;
                Some(self.index)
            }
        }
    }
}

impl ExactSizeIterator for SingleIndexed {
    fn len(&self) -> usize {
        1
    }
}

impl Indexing for Range<u16> {
    type OutputIndex = SingleIndexed;

    fn index(&self, index: Index) -> Result<Self::OutputIndex, MappingError> {
        let idx_mapped = self
            .start
            .checked_add(index)
            .ok_or(MappingError::NotInMappingRange)?;
        if idx_mapped >= self.end {
            return Err(MappingError::NotInMappingRange);
        }

        Ok(SingleIndexed::new(idx_mapped))
    }

    fn len(&self) -> usize {
        ExactSizeIterator::len(self)
    }
}

impl Indexing for &[LedId] {
    type OutputIndex = SingleIndexed;

    fn index(&self, index: Index) -> Result<Self::OutputIndex, MappingError> {
        Ok(SingleIndexed::new(self[usize::from(index)]))
    }

    fn len(&self) -> usize {
        self.deref().len()
    }
}

impl<const N: usize> Indexing for &[LedId; N] {
    type OutputIndex = SingleIndexed;

    fn index(&self, index: Index) -> Result<Self::OutputIndex, MappingError> {
        Ok(SingleIndexed::new(self[usize::from(index)]))
    }

    fn len(&self) -> usize {
        self.as_slice().len()
    }
}

impl<const N: usize> Indexing for [LedId; N] {
    type OutputIndex = SingleIndexed;

    fn index(&self, index: Index) -> Result<Self::OutputIndex, MappingError> {
        Ok(SingleIndexed::new(self[usize::from(index)]))
    }

    fn len(&self) -> usize {
        self.as_slice().len()
    }
}

#[cfg(test)]
mod test {

    use assert_matches::assert_matches;

    use super::*;

    #[test]
    fn test_bounded_indexed() {
        let indexed = [0, 1, 2, 3, 4, 5, 6, 7, 8];

        let bounded = BoundedIndexed::from_bounds(&indexed, Bound::None, Bound::Relative(2));

        assert_eq!(bounded.len(), 7);
        assert_eq!(*bounded.index(0).unwrap(), 0);
        assert_eq!(*bounded.index(6).unwrap(), 6);
        assert_matches!(bounded.index(7), Err(MappingError::NotInMappingRange));

        let bounded = BoundedIndexed::from_bounds(&indexed, Bound::Absolute(2), Bound::Relative(2));

        assert_eq!(bounded.len(), 5);
        assert_eq!(*bounded.index(0).unwrap(), 2);
        assert_eq!(*bounded.index(4).unwrap(), 6);
        assert_matches!(bounded.index(5), Err(MappingError::NotInMappingRange));

        let bounded = BoundedIndexed::from_bounds(&indexed, Bound::Absolute(2), Bound::Absolute(4));

        assert_eq!(bounded.len(), 3);
        assert_eq!(*bounded.index(0).unwrap(), 2);
        assert_eq!(*bounded.index(2).unwrap(), 4);
        assert_matches!(bounded.index(5), Err(MappingError::NotInMappingRange));

        let bounded = BoundedIndexed::from_range(&indexed, 2..7);
        assert_eq!(bounded.len(), 5);
        assert_eq!(*bounded.index(0).unwrap(), 2);
        assert_eq!(*bounded.index(4).unwrap(), 6);
        assert_matches!(bounded.index(5), Err(MappingError::NotInMappingRange));
    }

    #[test]
    fn test_split_mirrored_indexed() {
        let indexed = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let split = SplitMirroredIndexed::new(&indexed, UnevenBehavior::Exclude);

        assert_eq!(split.len(), 5);
        let mut first_indexes = split.index(0).unwrap();
        assert_eq!(first_indexes.len(), 2);
        assert_eq!(first_indexes.next().unwrap(), 0);
        assert_eq!(first_indexes.next().unwrap(), 9);
        assert_eq!(first_indexes.next(), None);

        let mut last_indexes = split.index(4).unwrap();
        assert_eq!(last_indexes.next().unwrap(), 4);
        assert_eq!(last_indexes.next().unwrap(), 5);

        let indexed = [0, 1, 2, 3, 4, 5, 6, 7, 8];
        let split_uneven = SplitMirroredIndexed::new(&indexed, UnevenBehavior::Exclude);

        assert_eq!(split_uneven.len(), 4);

        let mut first_indexes = split_uneven.index(0).unwrap();
        assert_eq!(first_indexes.next().unwrap(), 0);
        assert_eq!(first_indexes.next().unwrap(), 8);

        let mut last_indexes = split_uneven.index(3).unwrap();
        assert_eq!(last_indexes.next().unwrap(), 3);
        assert_eq!(last_indexes.next().unwrap(), 5);

        assert_matches!(split_uneven.index(4), Err(MappingError::NotInMappingRange));

        let split_uneven = SplitMirroredIndexed::new(&indexed, UnevenBehavior::ToLower);

        let mut last_indexes = split_uneven.index(4).unwrap();
        assert_eq!(last_indexes.next().unwrap(), 4);
        assert_eq!(last_indexes.next().unwrap(), 4);

        let split_uneven = SplitMirroredIndexed::new(&indexed, UnevenBehavior::ToUpper);

        let mut last_indexes = split_uneven.index(4).unwrap();
        assert_eq!(last_indexes.next().unwrap(), 4);
        assert_eq!(last_indexes.next().unwrap(), 4);
    }

    #[test]
    fn test_ext_trait() {
        let indexed = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

        let (h1, h2) = indexed.split_into_half(UnevenBehavior::Exclude);

        assert_eq!(h1.len(), 5);
        assert_eq!(h2.len(), 5);

        let h1_mirrored = h1.split_mirrored(UnevenBehavior::ToLower);
        let h2_reversed = h2.reversed();

        assert_eq!(h1_mirrored.len(), 3);
        assert_eq!(h2_reversed.len(), 5);

        let mut h1_mirrored_first = h1_mirrored.index(0).unwrap();
        assert_eq!(h1_mirrored_first.next().unwrap(), 0);
        assert_eq!(h1_mirrored_first.next().unwrap(), 4);

        let mut h1_mirrored_last = h1_mirrored.index(2).unwrap();
        assert_eq!(h1_mirrored_last.next().unwrap(), 2);
        assert_eq!(h1_mirrored_last.next().unwrap(), 2);

        let h1_mirrored_reversed = h1_mirrored.reversed();

        let mut h1_mirrored_reversed_first = h1_mirrored_reversed.index(0).unwrap();
        assert_eq!(h1_mirrored_reversed_first.next().unwrap(), 2);
        assert_eq!(h1_mirrored_reversed_first.next().unwrap(), 2);

        let mut h1_mirrored_reversed_last = h1_mirrored_reversed.index(2).unwrap();
        assert_eq!(h1_mirrored_reversed_last.next().unwrap(), 0);
        assert_eq!(h1_mirrored_reversed_last.next().unwrap(), 4);

        assert_eq!(*h2_reversed.index(0).unwrap(), 9);
        assert_eq!(*h2_reversed.index(4).unwrap(), 5);
    }

    #[test]
    fn test_circular_indexed() {
        let indexes = [0, 1, 2, 3, 4, 5, 6, 7, 8];
        let circle = CircularIndexed::new(&indexes, 2);

        assert_eq!(*circle.index(0).unwrap(), 2);
        assert_eq!(*circle.index(8).unwrap(), 1);

        let circle = CircularIndexed::new(&indexes, -2);

        assert_eq!(*circle.index(0).unwrap(), 7);
        assert_eq!(*circle.index(8).unwrap(), 6);
    }
}
