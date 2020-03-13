#![warn(clippy::all)]

use std::borrow::Borrow;
use std::borrow::Cow;
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

    pub fn intern_with<U, K>(
        &mut self,
        val: U,
        extract_key: impl FnOnce(&U) -> &K,
        make: impl FnOnce(U) -> T::Owned,
    ) -> Symbol<T>
    where
        T::Owned: Borrow<K>,
        K: ?Sized + Hash + Eq,
    {
        let idx = match self.pool.get_full(extract_key(&val)) {
            Some((idx, _)) => idx,
            None => self.pool.insert_full(make(val)).0,
        };

        Symbol::new(idx)
    }

    pub fn intern(&mut self, val: &T) -> Symbol<T> {
        self.intern_with(val, |val| *val, |val| val.to_owned())
    }

    pub fn intern_cow<'a>(&mut self, val: impl Into<Cow<'a, T>>) -> Symbol<T>
    where
        T: 'a,
    {
        self.intern_with(val.into(), |val| val.borrow(), |val| val.into_owned())
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
