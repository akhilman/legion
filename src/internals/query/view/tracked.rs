#![doc(hidden)]

use std::{any::TypeId, marker::PhantomData};

use super::{DefaultFilter, Fetch, IntoIndexableIter, IntoView, View};
use crate::internals::{
    iter::indexed::{IndexedIter, TrustedRandomAccess},
    permissions::Permissions,
    query::{
        filter::{component::ComponentFilter, passthrough::Passthrough, EntityFilterTuple},
        QueryResult,
    },
    storage::{
        archetype::Archetype,
        component::{Component, ComponentTypeId},
        ComponentMut, ComponentSliceMut, Components, Version,
    },
    subworld::ComponentAccess,
};

use super::write;

pub struct TrackedWrite<T>(PhantomData<T>);

impl<T> Default for TrackedWrite<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

unsafe impl<T: Send> Send for TrackedWrite<T> {}
unsafe impl<T> Sync for TrackedWrite<T> {}

impl<T: Component> DefaultFilter for TrackedWrite<T> {
    type Filter = EntityFilterTuple<ComponentFilter<T>, Passthrough>;
}

impl<T: Component> IntoView for TrackedWrite<T> {
    type View = Self;
}

impl<'data, T: Component> View<'data> for TrackedWrite<T> {
    type Element = <Self::Fetch as IntoIndexableIter>::Item;
    type Fetch = TrackedWriteFetch<'data, T>;
    type Iter = TrackedWriteIter<'data, T>;
    type Read = <write::Write<T> as View<'data>>::Read;
    type Write = <write::Write<T> as View<'data>>::Write;

    #[inline]
    fn validate() {
        <write::Write<T> as View<'data>>::validate()
    }

    #[inline]
    fn validate_access(access: &ComponentAccess) -> bool {
        <write::Write<T> as View<'data>>::validate_access(access)
    }

    #[inline]
    fn reads_types() -> Self::Read {
        <write::Write<T> as View<'data>>::reads_types()
    }

    #[inline]
    fn writes_types() -> Self::Write {
        <write::Write<T> as View<'data>>::writes_types()
    }

    #[inline]
    fn reads<D: Component>() -> bool {
        <write::Write<T> as View<'data>>::reads::<D>()
    }

    #[inline]
    fn writes<D: Component>() -> bool {
        <write::Write<T> as View<'data>>::writes::<D>()
    }

    #[inline]
    fn requires_permissions() -> Permissions<ComponentTypeId> {
        <write::Write<T> as View<'data>>::requires_permissions()
    }

    #[inline]
    unsafe fn fetch(
        components: &'data Components,
        archetypes: &'data [Archetype],
        query: QueryResult<'data>,
    ) -> Self::Iter {
        TrackedWriteIter(<write::Write<T> as View<'data>>::fetch(
            components, archetypes, query,
        ))
    }
}

type WriteIter<'a, T> = <write::Write<T> as View<'a>>::Iter;

#[doc(hidden)]
pub struct TrackedWriteIter<'a, T: Component>(WriteIter<'a, T>);

impl<'a, T: Component> Iterator for TrackedWriteIter<'a, T> {
    type Item = Option<TrackedWriteFetch<'a, T>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|maybe_fetch| maybe_fetch.map(|fetch| fetch.0.into()))
    }
}

#[doc(hidden)]
pub struct TrackedWriteFetch<'a, T: Component>(ComponentSliceMut<'a, T>);

impl<'a, T: Component> From<ComponentSliceMut<'a, T>> for TrackedWriteFetch<'a, T> {
    fn from(slice: ComponentSliceMut<'a, T>) -> Self {
        Self(slice)
    }
}

impl<'a, T: Component> IntoIndexableIter for TrackedWriteFetch<'a, T> {
    type Item = <Self as TrustedRandomAccess>::Item;
    type IntoIter = IndexedIter<Self>;
    fn into_indexable_iter(self) -> Self::IntoIter {
        IndexedIter::new(self)
    }
}

unsafe impl<'a, T: Component> TrustedRandomAccess for TrackedWriteFetch<'a, T> {
    type Item = ComponentMut<'a, T>;

    fn len(&self) -> usize {
        self.0.len()
    }
    unsafe fn get_unchecked(&mut self, i: usize) -> Self::Item {
        self.0.get_component_mut(i).expect("Component by index")
    }
    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.0.split_at(index);
        (left.into(), right.into())
    }
}

impl<'a, T: Component> Fetch for TrackedWriteFetch<'a, T> {
    type Data = &'a mut [T];

    #[inline]
    fn into_components(self) -> Self::Data {
        self.0.into_slice()
    }

    #[inline]
    fn find<C: 'static>(&self) -> Option<&[C]> {
        if TypeId::of::<C>() == TypeId::of::<T>() {
            // safety: C and T are the same type
            Some(unsafe { std::slice::from_raw_parts(self.0.as_ptr() as *const C, self.0.len()) })
        } else {
            None
        }
    }

    #[inline]
    fn find_mut<C: 'static>(&mut self) -> Option<&mut [C]> {
        if TypeId::of::<C>() == TypeId::of::<T>() {
            // safety: C and T are the same type
            Some(unsafe {
                std::slice::from_raw_parts_mut(self.0.as_mut_ptr() as *mut C, self.0.len())
            })
        } else {
            None
        }
    }

    #[inline]
    fn version<C: Component>(&self) -> Option<Version> {
        if TypeId::of::<C>() == TypeId::of::<T>() {
            Some(self.0.version())
        } else {
            None
        }
    }
}
