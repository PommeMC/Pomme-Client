use std::fmt::Debug;

use azalea_registry::identifier::Identifier;

use crate::core::id_map::IdMap;
use crate::resources::resource_key::ResourceKey;

pub trait Registry<T>: IdMap<T> + Debug {
    fn key(&self) -> ResourceKey<RegistryMarker<T>>;

    fn get_key(&self, thing: &T) -> Option<Identifier>;

    fn get_resource_key(&self, thing: &T) -> Option<ResourceKey<T>>;

    fn get_value_by_resource_key(&self, key: &ResourceKey<T>) -> Option<&T>;

    fn get_value_by_identifier(&self, key: &Identifier) -> Option<&T>;

    fn contains_identifier(&self, key: &Identifier) -> bool {
        self.get_value_by_identifier(key).is_some()
    }

    fn contains_resource_key(&self, key: &ResourceKey<T>) -> bool {
        self.get_value_by_resource_key(key).is_some()
    }

    fn get_value_or_throw(&self, key: &ResourceKey<T>) -> &T {
        match self.get_value_by_resource_key(key) {
            Some(value) => value,
            None => panic!("Missing key in {:?}: {}", self.key(), key),
        }
    }

    fn key_set(&self) -> Vec<Identifier>;

    fn registry_key_set(&self) -> Vec<ResourceKey<T>>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RegistryMarker<T>(std::marker::PhantomData<fn() -> T>);
