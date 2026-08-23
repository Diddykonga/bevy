use core::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use crate::{
    change_detection::{
        traits::{change_detection_impl, change_detection_mut_impl, impl_debug, impl_methods},
        MaybeLocation, MutUntyped,
    },
    component::{Component, ComponentId},
    entity::Entity,
    lifecycle::MUTATE,
    world::{CommandQueue, DeferredWorld, Mut, World},
};

pub(crate) struct EntityDeferredWorldMut<'w> {
    pub(crate) entity: Entity,
    pub(crate) world: DeferredWorld<'w>,
}

impl<'w> EntityDeferredWorldMut<'w> {
    unsafe fn trigger_mutate(&mut self, component: ComponentId) {
        // SAFETY: Caller ensures `component` is a valid [`ComponentId`].
        let has_hooks = unsafe {
            self.world
                .components()
                .get_info_unchecked(component)
                .hooks()
                .on_mutate
                .is_some()
        };
        assert!(has_hooks);
        let has_observers = self
            .world
            .observers()
            .try_get_observers(MUTATE)
            .is_some_and(|o| {
                !o.global_observers().is_empty()
                    || !o.entity_observers().is_empty()
                    || o.component_observers().contains_key(&component)
            });
        self.world.trigger_mutate::<false>(
            self.entity,
            [component].into(),
            has_hooks,
            has_observers,
            MaybeLocation::caller(),
        );
    }
}

/// Component Wrapper around mutable Component access, in order to facilitate Change Detection with Change [`Tick`]s via [`Mut`] and [`Mutate`] Hooks/Events.
pub struct MutCommands<'w, T: Component> {
    pub(crate) value: Mut<'w, T>,
    pub(crate) deferred: EntityDeferredWorldMut<'w>,
}

impl<'w, T: Component> MutCommands<'w, T> {
    /// Triggers [`Mutate`] Hooks and Events
    pub fn trigger_mutate(&mut self) {
        self.deref_mut();
    }
}

impl<'w, T: Component> Deref for MutCommands<'w, T> {
    type Target = Mut<'w, T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Component> DerefMut for MutCommands<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let Some(component) = self.deferred.world.component_id::<T>() {
            // SAFETY: `component` is Some, so it is a valid [`ComponentId`].
            unsafe { self.deferred.trigger_mutate(component) };
        }
        &mut self.value
    }
}

impl<T: Component> AsMut<T> for MutCommands<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

impl<'w, T: Component> From<MutCommands<'w, T>> for Mut<'w, T> {
    fn from(mut_c: MutCommands<'w, T>) -> Self {
        Self {
            value: (mut_c.value.value),
            ticks: (mut_c.value.ticks),
        }
    }
}

impl<T: Component + Debug> Debug for MutCommands<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("MutCommands<'w, T>")
            .field(&self.value.value)
            .finish()
    }
}

struct MutCommandsUntyped<'w> {
    value: MutUntyped<'w>,
    deferred: EntityDeferredWorldMut<'w>,
}
