use std::borrow::Borrow;
use std::hash::BuildHasherDefault;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Index;

use indexmap::IndexSet;
use rustc_hash::FxHasher;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Symbol<T: ToOwned + ?Sized> {
    idx: usize,
    marker: PhantomData<fn(&T::Owned) -> &T>,
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

#[derive(Default)]
pub struct Interner<T: ToOwned + ?Sized> {
    pool: FxIndexSet<T::Owned>,
}

impl<T: ToOwned + ?Sized> Interner<T>
where
    T: Hash + Eq,
    T::Owned: Hash + Eq,
{
    pub fn new() -> Self {
        Self {
            pool: FxIndexSet::with_capacity_and_hasher(0, Default::default()),
        }
    }

    pub fn intern<U>(&mut self, val: U) -> Symbol<T>
    where
        U: Borrow<T> + Into<T::Owned>,
    {
        let idx = match self.pool.get_full(val.borrow()) {
            Some((idx, _)) => idx,
            None => self.pool.insert_full(val.into()).0,
        };

        Symbol::new(idx)
    }

    pub fn resolve(&self, sym: Symbol<T>) -> Option<&T> {
        self.pool.get_index(sym.idx).map(|val| val.borrow())
    }
}

impl<T: ToOwned + ?Sized> Index<Symbol<T>> for Interner<T>
where
    T: Hash + Eq,
    T::Owned: Hash + Eq,
{
    type Output = T;

    fn index(&self, sym: Symbol<T>) -> &T {
        self.resolve(sym).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_str() {
        let mut interner = Interner::<str>::new();

        let hi = interner.intern("hi");
        let bye = interner.intern("bye");
        let hi2 = interner.intern("hi");

        assert_eq!(hi, hi2);
        assert_ne!(hi, bye);
        assert_eq!(&interner[hi], "hi");
        assert_eq!(&interner[bye], "bye");
    }
}
