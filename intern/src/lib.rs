#![warn(clippy::all)]

use std::borrow::Borrow;
use std::hash::BuildHasherDefault;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Index;

use indexmap::IndexSet;
use rustc_hash::FxHasher;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol<T: ?Sized> {
    idx: usize,
    marker: PhantomData<fn() -> T>,
}

impl<T: ?Sized> Symbol<T> {
    fn new(idx: usize) -> Self {
        Self {
            idx,
            marker: PhantomData,
        }
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
