//! A simple interner for types implementing `ToOwned`.

use std::borrow::{Borrow, Cow};
use std::hash::BuildHasherDefault;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Index;

use indexmap::IndexSet;
use rustc_hash::FxHasher;

/// Opaque type used to refer to interned data.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Symbol<T: ToOwned + ?Sized> {
    idx: usize,
    marker: PhantomData<fn(&Interner<T>) -> &T>,
}

impl<T: ToOwned + ?Sized> Symbol<T> {
    fn new(idx: usize) -> Self {
        Self {
            idx,
            marker: PhantomData,
        }
    }
}

// Implement manually because deriving requires all generic paramaters to be `Copy` as well.
impl<T: ToOwned + ?Sized> Copy for Symbol<T> {}

impl<T: ToOwned + ?Sized> Clone for Symbol<T> {
    fn clone(&self) -> Self {
        *self
    }
}

type FxIndexSet<T> = IndexSet<T, BuildHasherDefault<FxHasher>>;

/// A simple interner for types implementing `ToOwned`.
#[derive(Default)]
pub struct Interner<T: ToOwned + ?Sized> {
    pool: FxIndexSet<T::Owned>,
}

impl<T: ToOwned + ?Sized> Interner<T>
where
    T: Hash + Eq,
    T::Owned: Hash + Eq,
{
    /// Creates a new, empty interner.
    pub fn new() -> Self {
        Self {
            pool: FxIndexSet::with_capacity_and_hasher(0, Default::default()),
        }
    }

    /// Interns the provided value, upgrading it to an owned one if necessary.
    ///
    /// Returns a symbol uniquely identifying the interned value. If the same value is interned
    /// multiple times, the same symbol will be returned every time.
    pub fn intern(&mut self, val: &T) -> Symbol<T> {
        self.intern_cow(Cow::Borrowed(val))
    }

    /// Interns the provided value, storing it as an owned one if necessary.
    ///
    /// This method enables less potential allocations than [`intern()`](#method.intern) if `val` is
    /// already owned.
    ///
    /// Returns a symbol uniquely identifying the interned value. If the same value is interned
    /// multiple times, the same symbol will be returned every time.
    pub fn intern_cow(&mut self, val: Cow<T>) -> Symbol<T> {
        let idx = match self.pool.get_full(&*val) {
            Some((idx, _)) => idx,
            None => self.pool.insert_full(val.into_owned()).0,
        };

        Symbol::new(idx)
    }

    /// Resolves the symbol to its interned content.
    ///
    /// # Panics
    ///
    /// Panics if `sym` has no associated data in this interner. This can happen if it came from a
    /// different interner.
    pub fn resolve(&self, sym: Symbol<T>) -> &T {
        self.pool
            .get_index(sym.idx)
            .expect("symbol used with wrong interner")
            .borrow()
    }
}

impl<T: ToOwned + ?Sized> Index<Symbol<T>> for Interner<T>
where
    T: Hash + Eq,
    T::Owned: Hash + Eq,
{
    type Output = T;

    fn index(&self, sym: Symbol<T>) -> &T {
        self.resolve(sym)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_str() {
        let mut interner = Interner::new();

        let hi = interner.intern("hi");
        let bye = interner.intern("bye");
        let hi2 = interner.intern("hi");

        assert_eq!(hi, hi2);
        assert_ne!(hi, bye);
        assert_eq!(&interner[hi], "hi");
        assert_eq!(&interner[bye], "bye");
    }
}
