use std::fmt;
use std::marker::PhantomData;
use std::sync::LazyLock;

use azalea_registry::identifier::Identifier;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ResourceKey<T> {
    registry_name: Identifier,
    identifier: Identifier,
    _marker: PhantomData<fn() -> T>,
}

impl<T> ResourceKey<T> {
    pub fn create(registry_name: Identifier, identifier: Identifier) -> Self {
        Self {
            registry_name,
            identifier,
            _marker: PhantomData,
        }
    }

    pub fn is_for<E>(&self, registry: &ResourceKey<E>) -> bool {
        self.registry_name == registry.identifier
    }

    pub fn cast<E>(&self, registry: &ResourceKey<E>) -> Option<ResourceKey<E>> {
        self.is_for(registry)
            .then(|| ResourceKey::<E>::create(self.registry_name.clone(), self.identifier.clone()))
    }

    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }

    pub fn registry(&self) -> &Identifier {
        &self.registry_name
    }

    pub fn registry_key(&self) -> ResourceKey<Registry<T>> {
        ResourceKey::<Registry<T>>::create_registry_key(self.registry_name.clone())
    }
}

impl<T> fmt::Debug for ResourceKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceKey")
            .field("registry_name", &self.registry_name)
            .field("identifier", &self.identifier)
            .finish()
    }
}

impl<T> fmt::Display for ResourceKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ResourceKey[{} / {}]",
            self.registry_name, self.identifier
        )
    }
}

pub struct Registry<T>(PhantomData<fn() -> T>);

impl<T> ResourceKey<Registry<T>> {
    pub fn create_registry_key(identifier: Identifier) -> Self {
        Self::create(ROOT_REGISTRY_NAME.clone(), identifier)
    }
}

pub static ROOT_REGISTRY_NAME: LazyLock<Identifier> =
    LazyLock::new(|| Identifier::new("minecraft:root"));
