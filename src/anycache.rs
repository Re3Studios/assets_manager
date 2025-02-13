//! This module defines `AnyCache` and many traits to reduce code redondancy.
//!
//! There are 6 traits here: `RawCache`, `Cache`, `CacheExt` and variants with
//! source. The goal of this is to have an object-safe cache trait to use in
//! `AnyCache`, while not losing the ability to use caches without virtual
//! calls.
//!
//! - The `Cache` (and `CacheWithSource`) variants are the central ones, and are
//!   designed to be object safe.
//! - The `RawCache` variant is there to ease implementations of `Cache`
//!   without repeating code.
//! - The `CacheExt` variant adds generics on top of `Cache` to ease the use of
//!   `Cache`'s methods.

use std::{any::TypeId, borrow::Cow, fmt, io};

use crate::{
    asset::DirLoadable,
    cache::AssetMap,
    entry::{CacheEntry, CacheEntryInner},
    source::{DirEntry, Source},
    utils::{OwnedKey, Private},
    Compound, DirHandle, Error, Handle, SharedString, Storable,
};

#[cfg(feature = "hot-reloading")]
use crate::hot_reloading::{records, Dependencies, HotReloader};

/// TODO
#[derive(Clone, Copy)]
pub struct AnyCache<'a> {
    cache: &'a dyn CacheWithSource,
}

/// TODO
#[derive(Clone, Copy)]
struct AnySource<'a> {
    cache: &'a dyn CacheWithSource,
}

impl Source for AnySource<'_> {
    #[inline]
    fn read(&self, id: &str, ext: &str) -> io::Result<Cow<[u8]>> {
        self.cache.read(id, ext)
    }

    #[inline]
    fn read_dir(&self, id: &str, f: &mut dyn FnMut(DirEntry)) -> io::Result<()> {
        self.cache.read_dir(id, f)
    }

    #[inline]
    fn exists(&self, entry: DirEntry) -> bool {
        self.cache.exists(entry)
    }
}

impl<'a> AnyCache<'a> {
    /// TODO
    #[inline]
    pub fn source(self) -> impl Source + 'a {
        AnySource { cache: self.cache }
    }

    #[cfg(feature = "hot-reloading")]
    #[inline]
    pub(crate) fn reloader(&self) -> Option<&'a HotReloader> {
        self.cache.reloader()
    }

    /// TODO
    #[inline]
    pub fn get_cached<A: Storable>(self, id: &str) -> Option<Handle<'a, A>> {
        self.cache._get_cached(id)
    }

    /// TODO
    #[inline]
    pub fn get_or_insert<A: Storable>(self, id: &str, default: A) -> Handle<'a, A> {
        self.cache._get_or_insert(id, default)
    }

    /// TODO
    #[inline]
    pub fn contains<A: Storable>(self, id: &str) -> bool {
        self.cache._contains::<A>(id)
    }

    /// TODO
    #[inline]
    pub fn load<A: Compound>(self, id: &str) -> Result<Handle<'a, A>, Error> {
        self.cache._load(id)
    }

    /// TODO
    #[inline]
    pub fn load_expect<A: Compound>(self, id: &str) -> Handle<'a, A> {
        self.cache._load_expect(id)
    }

    /// TODO
    #[inline]
    pub fn load_dir<A: DirLoadable>(
        self,
        id: &str,
        recursive: bool,
    ) -> Result<DirHandle<'a, A>, Error> {
        self.cache._load_dir(id, recursive)
    }

    /// TODO
    #[inline]
    pub fn load_owned<A: Compound>(self, id: &str) -> Result<A, Error> {
        self.cache._load_owned(id)
    }

    /// TODO
    #[inline]
    pub fn no_record<T, F: FnOnce() -> T>(self, f: F) -> T {
        #[cfg(feature = "hot-reloading")]
        {
            records::no_record(f)
        }

        #[cfg(not(feature = "hot-reloading"))]
        {
            f()
        }
    }

    #[cfg(feature = "hot-reloading")]
    #[inline]
    pub(crate) fn record_load<A: Compound>(
        self,
        id: &str,
    ) -> Result<(A, Dependencies), crate::BoxedError> {
        let (asset, records) = if let Some(reloader) = self.reloader() {
            records::record(reloader, || A::load(self, id))
        } else {
            (A::load(self, id), Dependencies::empty())
        };

        Ok((asset?, records))
    }
}

impl fmt::Debug for AnyCache<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnyCache").finish_non_exhaustive()
    }
}

pub(crate) trait Cache {
    fn assets(&self) -> &AssetMap;

    #[cfg(feature = "hot-reloading")]
    fn reloader(&self) -> Option<&HotReloader>;

    fn get_cached_entry_inner(
        &self,
        id: &str,
        type_id: TypeId,
        hot_reloaded: bool,
    ) -> Option<CacheEntryInner>;

    fn contains(&self, id: &str, type_id: TypeId) -> bool;
}

pub(crate) trait CacheWithSource: Cache {
    fn read(&self, id: &str, ext: &str) -> io::Result<Cow<[u8]>>;

    fn read_dir(&self, id: &str, f: &mut dyn FnMut(DirEntry)) -> io::Result<()>;

    fn exists(&self, entry: DirEntry) -> bool;

    fn load_entry(
        &self,
        id: &str,
        type_id: TypeId,
        hot_reloaded: bool,
        load: fn(AnyCache, SharedString) -> Result<CacheEntry, Error>,
    ) -> Result<CacheEntryInner, Error>;
}

impl Cache for &dyn CacheWithSource {
    #[inline]
    fn assets(&self) -> &AssetMap {
        (**self).assets()
    }

    #[cfg(feature = "hot-reloading")]
    #[inline]
    fn reloader(&self) -> Option<&HotReloader> {
        (**self).reloader()
    }

    #[inline]
    fn get_cached_entry_inner(
        &self,
        id: &str,
        type_id: TypeId,
        hot_reloaded: bool,
    ) -> Option<CacheEntryInner> {
        (*self).get_cached_entry_inner(id, type_id, hot_reloaded)
    }

    #[inline]
    fn contains(&self, id: &str, type_id: TypeId) -> bool {
        (*self).contains(id, type_id)
    }
}

impl CacheWithSource for &dyn CacheWithSource {
    #[inline]
    fn read(&self, id: &str, ext: &str) -> io::Result<Cow<[u8]>> {
        (*self).read(id, ext)
    }

    #[inline]
    fn read_dir(&self, id: &str, f: &mut dyn FnMut(DirEntry)) -> io::Result<()> {
        (*self).read_dir(id, f)
    }

    #[inline]
    fn exists(&self, entry: DirEntry) -> bool {
        (**self).exists(entry)
    }

    #[inline]
    fn load_entry(
        &self,
        id: &str,
        type_id: TypeId,
        hot_reloaded: bool,
        load: fn(AnyCache, SharedString) -> Result<CacheEntry, Error>,
    ) -> Result<CacheEntryInner, Error> {
        (*self).load_entry(id, type_id, hot_reloaded, load)
    }
}

pub(crate) trait RawCache: Sized {
    fn assets(&self) -> &AssetMap;

    #[cfg(feature = "hot-reloading")]
    fn reloader(&self) -> Option<&HotReloader>;
}

pub(crate) trait RawCacheWithSource: RawCache {
    type Source: Source;

    fn get_source(&self) -> &Self::Source;

    #[cold]
    fn add_asset(
        &self,
        id: &str,
        type_id: TypeId,
        load: fn(AnyCache, SharedString) -> Result<CacheEntry, Error>,
    ) -> Result<CacheEntryInner, Error> {
        log::trace!("Loading \"{}\"", id);

        let id = SharedString::from(id);
        let cache = AnyCache { cache: self };
        let entry = load(cache, id.clone())?;

        Ok(self.assets().insert(id, type_id, entry))
    }
}

impl<T: RawCache> Cache for T {
    #[inline]
    fn assets(&self) -> &AssetMap {
        self.assets()
    }

    #[cfg(feature = "hot-reloading")]
    #[inline]
    fn reloader(&self) -> Option<&HotReloader> {
        self.reloader()
    }

    fn get_cached_entry_inner(
        &self,
        id: &str,
        type_id: TypeId,
        _hot_reloaded: bool,
    ) -> Option<CacheEntryInner> {
        #[cfg(feature = "hot-reloading")]
        if _hot_reloaded {
            if let Some(reloader) = self.reloader() {
                let (key, entry) = match self.assets().get_key_entry(id, type_id) {
                    Some((key, entry)) => (key, Some(entry)),
                    None => (OwnedKey::new_with(id.into(), type_id), None),
                };
                records::add_record(reloader, key);
                return entry;
            }
        }

        self.assets().get_entry(id, type_id)
    }

    #[inline]
    fn contains(&self, id: &str, type_id: TypeId) -> bool {
        self.assets().contains_key(id, type_id)
    }
}

impl<T: RawCacheWithSource> CacheWithSource for T {
    fn read(&self, id: &str, ext: &str) -> io::Result<Cow<[u8]>> {
        self.get_source().read(id, ext)
    }

    fn read_dir(&self, id: &str, f: &mut dyn FnMut(DirEntry)) -> io::Result<()> {
        self.get_source().read_dir(id, f)
    }

    fn exists(&self, entry: DirEntry) -> bool {
        self.get_source().exists(entry)
    }

    fn load_entry(
        &self,
        id: &str,
        type_id: TypeId,
        hot_reloaded: bool,
        load: fn(AnyCache, SharedString) -> Result<CacheEntry, Error>,
    ) -> Result<CacheEntryInner, Error> {
        match self.get_cached_entry_inner(id, type_id, hot_reloaded) {
            Some(entry) => Ok(entry),
            None => self.add_asset(id, type_id, load),
        }
    }
}

pub(crate) trait CacheExt: Cache {
    #[inline]
    fn _get_cached<A: Storable>(&self, id: &str) -> Option<Handle<A>> {
        Some(self._get_cached_entry::<A>(id)?.handle())
    }

    #[inline]
    fn _get_cached_entry<A: Storable>(&self, id: &str) -> Option<CacheEntryInner> {
        self.get_cached_entry_inner(id, TypeId::of::<A>(), A::HOT_RELOADED)
    }

    #[cold]
    fn add_any<A: Storable>(&self, id: &str, asset: A) -> CacheEntryInner {
        let id = SharedString::from(id);
        let entry = CacheEntry::new(asset, id.clone());

        self.assets().insert(id, TypeId::of::<A>(), entry)
    }

    fn _get_or_insert<A: Storable>(&self, id: &str, default: A) -> Handle<A> {
        let entry = match self._get_cached_entry::<A>(id) {
            Some(entry) => entry,
            None => self.add_any(id, default),
        };

        entry.handle()
    }

    #[inline]
    fn _contains<A: Storable>(&self, id: &str) -> bool {
        self.contains(id, TypeId::of::<A>())
    }
}

impl<T: Cache + ?Sized> CacheExt for T {}

pub(crate) trait CacheWithSourceExt: CacheWithSource + CacheExt {
    fn _as_any_cache(&self) -> AnyCache;

    fn _load<A: Compound>(&self, id: &str) -> Result<Handle<A>, Error> {
        let entry = self.load_entry(
            id,
            TypeId::of::<A>(),
            A::HOT_RELOADED,
            A::_load_and_record_entry::<Private>,
        )?;
        Ok(entry.handle())
    }

    #[inline]
    #[track_caller]
    fn _load_expect<A: Compound>(&self, id: &str) -> Handle<A> {
        #[cold]
        #[track_caller]
        fn expect_failed(err: Error) -> ! {
            panic!(
                "Failed to load essential asset \"{}\": {}",
                err.id(),
                err.reason()
            )
        }

        // Do not use `unwrap_or_else` as closures do not have #[track_caller]
        match self._load(id) {
            Ok(h) => h,
            Err(err) => expect_failed(err),
        }
    }

    #[inline]
    fn _get_cached_dir<A: DirLoadable>(&self, id: &str, recursive: bool) -> Option<DirHandle<A>> {
        Some(if recursive {
            let handle = self._get_cached(id)?;
            DirHandle::new_rec(handle, self._as_any_cache())
        } else {
            let handle = self._get_cached(id)?;
            DirHandle::new(handle, self._as_any_cache())
        })
    }

    #[inline]
    fn _load_dir<A: DirLoadable>(&self, id: &str, recursive: bool) -> Result<DirHandle<A>, Error> {
        Ok(if recursive {
            let handle = self._load(id)?;
            DirHandle::new_rec(handle, self._as_any_cache())
        } else {
            let handle = self._load(id)?;
            DirHandle::new(handle, self._as_any_cache())
        })
    }

    fn _load_owned<A: Compound>(&self, id: &str) -> Result<A, Error> {
        let id = SharedString::from(id);
        let asset = A::_load_and_record::<Private>(self._as_any_cache(), &id);

        #[cfg(feature = "hot-reloading")]
        if A::HOT_RELOADED {
            if let Some(reloader) = self.reloader() {
                let key = OwnedKey::new::<A>(id);
                records::add_record(reloader, key);
            }
        }

        asset
    }
}

impl<T: CacheWithSource> CacheWithSourceExt for T {
    #[inline]
    fn _as_any_cache(&self) -> AnyCache {
        AnyCache { cache: self }
    }
}

impl CacheWithSourceExt for dyn CacheWithSource + '_ {
    #[inline]
    fn _as_any_cache(&self) -> AnyCache {
        AnyCache { cache: self }
    }
}
