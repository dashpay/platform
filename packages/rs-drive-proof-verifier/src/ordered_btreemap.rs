//! [BTreeMap] that can be ordered in either ascending or descending order.
use bincode::{Decode, Encode};
use std::collections::btree_map::{Iter, Keys};
use std::{collections::BTreeMap, iter::Rev};

/// Iterator that can be either forward or reverse.
///
/// Used to iterate over items in a map in either ascending or descending order.
pub enum OrderedIterator<I: DoubleEndedIterator> {
    /// Items are iterated in ascending order.
    Ascending(I),
    /// Items are iterated in descending order.
    Descending(Rev<I>),
}

impl<I: DoubleEndedIterator> Iterator for OrderedIterator<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            OrderedIterator::Ascending(iter) => iter.next(),
            OrderedIterator::Descending(iter) => iter.next(),
        }
    }
}

/// BTreeMap that can be ordered in either ascending or descending order.
#[derive(Clone, Debug, Default, derive_more::From)]
#[cfg_attr(feature = "mocks", derive(Encode, Decode))]

pub struct OrderedBTreeMap<K: Ord, V> {
    /// The inner map.
    inner: BTreeMap<K, V>,
    /// When set to true, the map will be iterated in ascending order. Otherwise, it will be
    /// iterated in descending order.
    order_ascending: bool,
}

impl<K: Ord, V> OrderedBTreeMap<K, V> {
    /// Create a new OrderedBTreeMap with provided ordering.
    pub fn new(order_ascending: bool) -> Self {
        Self {
            inner: BTreeMap::new(),
            order_ascending,
        }
    }

    /// Creates a new OrderedBTreeMap from a BTreeMap with the provided ordering.
    ///
    /// This function consumes the provided map to avoid cloning it, thus it is more efficient than
    /// creating a new OrderedBTreeMap and then extending it with the provided map.
    pub fn from_btreemap(map: BTreeMap<K, V>, order_ascending: bool) -> Self {
        Self {
            inner: map,
            order_ascending,
        }
    }

    /// Returns an iterator over the map.
    ///
    /// See [BTreemap::iter()] for more information.
    pub fn iter(&self) -> OrderedIterator<Iter<'_, K, V>> {
        if self.order_ascending {
            OrderedIterator::Ascending(self.inner.iter())
        } else {
            OrderedIterator::Descending(self.inner.iter().rev())
        }
    }

    /// Returns true if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Gets an iterator over the keys of the map, in sorted order.
    pub fn keys(&self) -> OrderedIterator<Keys<'_, K, V>> {
        if self.order_ascending {
            OrderedIterator::Ascending(self.inner.keys())
        } else {
            OrderedIterator::Descending(self.inner.keys().rev())
        }
    }

    /// Gets all values in the map as a vector, ordered by key.
    pub fn values_vec(&self) -> Vec<&V> {
        self.iter().map(|(_k, v)| v).collect()
    }
}

impl<K: Ord, V> From<OrderedBTreeMap<K, V>> for BTreeMap<K, V> {
    fn from(value: OrderedBTreeMap<K, V>) -> Self {
        value.inner
    }
}

impl<K: Ord, V> Extend<(K, V)> for OrderedBTreeMap<K, V> {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = (K, V)>,
    {
        self.inner.extend(iter)
    }
}

impl<K: Ord, V> From<BTreeMap<K, V>> for OrderedBTreeMap<K, V> {
    fn from(value: BTreeMap<K, V>) -> Self {
        Self {
            inner: value,
            order_ascending: true, // btreemap is always ordered in ascending order
        }
    }
}

impl<K: Ord, V> IntoIterator for OrderedBTreeMap<K, V> {
    type Item = (K, V);
    type IntoIter = OrderedIterator<<BTreeMap<K, V> as IntoIterator>::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        let iter = self.inner.into_iter();
        if self.order_ascending {
            OrderedIterator::Ascending(iter)
        } else {
            OrderedIterator::Descending(iter.rev())
        }
    }
}
