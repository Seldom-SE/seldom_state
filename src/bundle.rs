use std::{
    fmt::{Debug, Formatter, Result},
    marker::PhantomData,
};

use bevy::{ecs::system::EntityCommands, prelude::*};

pub trait Insert: 'static + Send + Sync {
    fn insert(&self, entity: &mut EntityCommands);
    fn dyn_clone(&self) -> Box<dyn Insert>;
}

impl Debug for dyn Insert {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "dyn Insert")
    }
}

impl<T: Bundle + Clone> Insert for T {
    fn insert(&self, entity: &mut EntityCommands) {
        entity.insert(self.clone());
    }

    fn dyn_clone(&self) -> Box<dyn Insert> {
        Box::new(self.clone())
    }
}

pub(crate) trait Remove: Send + Sync {
    fn remove(&self, entity: &mut EntityCommands);
    fn dyn_clone(&self) -> Box<dyn Remove>;
}

impl Debug for dyn Remove {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "dyn Remove")
    }
}

pub(crate) struct Remover<B: Bundle>(PhantomData<B>);

impl<B: Bundle + Clone> Remove for B {
    fn remove(&self, entity: &mut EntityCommands) {
        entity.remove::<B>();
    }

    fn dyn_clone(&self) -> Box<dyn Remove> {
        Box::new(self.clone())
    }
}

impl<B: Bundle> Remove for Remover<B> {
    fn remove(&self, entity: &mut EntityCommands) {
        entity.remove::<B>();
    }

    fn dyn_clone(&self) -> Box<dyn Remove> {
        Box::new(B::remover())
    }
}

pub(crate) trait Removable {
    type B: Bundle;

    fn remover() -> Remover<Self::B>;
}

impl<B: Bundle> Removable for B {
    type B = Self;

    fn remover() -> Remover<Self::B> {
        Remover(default())
    }
}
