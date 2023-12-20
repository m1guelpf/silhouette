#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

//! ## About Silhouette
//!
//! Silhouette implements a simple service container in Rust for dependency injection.
//! It not only provides a [`Container`] struct for local usage, but also a static interface (under [`facade::Container`]) for easily managing dependencies throughout your application.
//! It's heavily inspired by [Laravel's Service Container](https://laravel.com/docs/container).
//!
//! ## Usage
//!
//! ```rust
//! use silhouette::facade::Container;
//!
//! # #[derive(Debug, Clone)]
//! struct DBPool {}
//! # impl DBPool {
//! #     fn new() -> Self {
//! #         Self {}
//! #     }
//! #
//! #     fn get_conn(&self) -> DBConnection {
//! #         DBConnection {}
//! #     }
//! # }
//! struct DBConnection {}
//!
//! # #[test]
//! # fn test() -> Result<(), silhouette::Error> {
//! // will always use the same pool
//! Container::singleton(&|_| DBPool::new())?;
//!
//! // will resolve a new connection each time
//! Container::bind(&|container| -> DBConnection {
//!     let shared_pool = container.resolve::<DBPool>().unwrap();
//!
//!     shared_pool.get_conn()
//! })?;
//!
//! // somewhere else in your app...
//! let connection: DBConnection = Container::resolve()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Features
//!
//! - `nightly` - Automatically resolves types that implement [`Default`]. Requires the nightly compiler.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{OnceLock, RwLock},
};
#[cfg(feature = "nightly")]
use try_default::TryDefault;

pub(crate) static SERVICE_CONTAINER: OnceLock<RwLock<Container>> = OnceLock::new();

/// A static interface for the service container.
pub mod facade;

/// The service container.
pub struct Container {
    #[allow(clippy::type_complexity)]
    /// The container's bindings.
    bindings: HashMap<TypeId, Box<(dyn Fn(&Self) -> Box<dyn Any> + Sync + Send)>>,
    /// The container's shared instances.
    instances: HashMap<TypeId, Box<(dyn Fn() -> Box<dyn Any> + Sync + Send)>>,
    /// The container's scoped instances.
    scoped_instances: Vec<TypeId>,
}

impl Container {
    /// Create a new instance of the container.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            instances: HashMap::new(),
            scoped_instances: Vec::new(),
        }
    }

    /// Get the global instance of the container.
    pub fn get_instance() -> &'static RwLock<Self> {
        SERVICE_CONTAINER.get_or_init(|| RwLock::new(Self::new()))
    }

    /// Register a binding with the container.
    pub fn bind<T: 'static>(&mut self, factory: impl Fn(&Self) -> T + 'static + Sync + Send) {
        self.instances.remove(&TypeId::of::<T>());

        self.bindings.insert(
            TypeId::of::<T>(),
            Box::new(move |container: &Self| {
                let result = factory(container);

                Box::new(result) as Box<dyn Any>
            }),
        );
    }

    /// Register a binding if it hasn't already been registered.
    pub fn bind_if<T: 'static>(&mut self, factory: impl Fn(&Self) -> T + 'static + Sync + Send) {
        if !self.bindings.contains_key(&TypeId::of::<T>()) {
            self.bind(factory);
        }
    }

    /// Register a scoped binding in the container.
    pub fn scoped<T: 'static + Clone + Send + Sync>(
        &mut self,
        factory: &(impl Fn(&Self) -> T + 'static),
    ) {
        self.scoped_instances.push(TypeId::of::<T>());

        self.singleton(factory);
    }

    /// Register a scoped binding if it hasn't already been registered.
    pub fn scoped_if<T: 'static + Clone + Send + Sync>(
        &mut self,
        factory: &(impl Fn(&Self) -> T + 'static),
    ) {
        if !self.scoped_instances.contains(&TypeId::of::<T>()) {
            self.scoped(factory);
        }
    }

    /// Register a shared binding in the container.
    pub fn singleton<T: 'static + Clone + Send + Sync>(
        &mut self,
        factory: &(impl Fn(&Self) -> T + 'static),
    ) {
        let result = factory(self);

        self.instances.insert(
            TypeId::of::<T>(),
            Box::new(move || Box::new(result.clone()) as Box<dyn Any + Send + Sync>),
        );
    }

    /// Register a shared binding if it hasn't already been registered.
    pub fn singleton_if<T: 'static + Clone + Send + Sync>(
        &mut self,
        factory: &(impl Fn(&Self) -> T + 'static),
    ) {
        if !self.instances.contains_key(&TypeId::of::<T>()) {
            self.singleton(factory);
        }
    }

    /// Resolve the given type from the container.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested type cannot be found or if the requested type cannot be cast from the binding.
    pub fn resolve<T: 'static>(&self) -> Result<T, Error> {
        let type_id = TypeId::of::<T>();

        if let Some(instance) = self.instances.get(&type_id) {
            return instance()
                .downcast::<T>()
                .map(|i| *i)
                .map_err(|_| Error::CastFailed);
        };

        if let Some(binding) = self.bindings.get(&type_id) {
            return binding(self)
                .downcast::<T>()
                .map(|b| *b)
                .map_err(|_| Error::CastFailed);
        };

        try_default_if_enabled().ok_or(Error::NotFound)
    }

    /// Clear all of the scoped instances from the container.
    pub fn forget_scoped_instances(&mut self) {
        for instance in &self.scoped_instances {
            self.instances.remove(instance);
        }
    }

    /// Flush the container of all bindings and resolved instances.
    pub fn flush(&mut self) {
        self.bindings.clear();
        self.instances.clear();
        self.scoped_instances.clear();
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "nightly"))]
const fn try_default_if_enabled<T>() -> Option<T> {
    None
}

#[cfg(feature = "nightly")]
fn try_default_if_enabled<T>() -> Option<T> {
    T::try_default()
}

/// An error that can occur when interacting with the container.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Binding not found.
    #[error("Binding not found")]
    NotFound,

    /// Failed to cast binding to requested type.
    #[error("Failed to cast binding to requested type")]
    CastFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[derive(Debug, Clone, PartialEq)]
    struct TestDependency {
        value: String,
    }

    #[test]
    fn can_register_a_binding() {
        let mut container = Container::new();

        container.bind(|_: &Container| TestDependency {
            value: "Hello, world!".to_string(),
        });

        assert_eq!(container.bindings.len(), 1);
        assert_eq!(container.instances.len(), 0);
    }

    #[test]
    fn can_register_a_singleton() {
        let mut container = Container::new();

        container.singleton(&|_: &Container| TestDependency {
            value: "Hello, world!".to_string(),
        });

        assert_eq!(container.bindings.len(), 0);
        assert_eq!(container.instances.len(), 1);
    }

    #[test]
    fn can_register_a_scoped_binding() {
        let mut container = Container::new();

        container.scoped(&|_: &Container| TestDependency {
            value: "Hello, world!".to_string(),
        });

        assert_eq!(container.bindings.len(), 0);
        assert_eq!(container.instances.len(), 1);
        assert_eq!(container.scoped_instances.len(), 1);
    }

    #[test]
    fn registering_a_binding_clears_previous_singleton() {
        let mut container = Container::new();

        container.singleton(&|_: &Container| TestDependency {
            value: "Hello, world!".to_string(),
        });

        container.bind(|_: &Container| TestDependency {
            value: "Goodbye, world!".to_string(),
        });

        assert_eq!(container.bindings.len(), 1);
        assert_eq!(container.instances.len(), 0);

        assert_eq!(
            container.resolve::<TestDependency>(),
            Ok(TestDependency {
                value: "Goodbye, world!".to_string()
            })
        );
    }

    #[test]
    fn can_retrieve_a_registered_binding() {
        let mut container = Container::new();

        container.bind(|_: &Container| TestDependency {
            value: "Hello, world!".to_string(),
        });

        let result = container.resolve::<TestDependency>().unwrap();

        assert_eq!(result.value, "Hello, world!");
    }

    #[test]
    fn can_retrieve_a_registered_scoped_binding_until_flush() {
        let mut container = Container::new();

        container.scoped(&|_: &Container| TestDependency {
            value: "Hello, world!".to_string(),
        });

        assert_eq!(
            container.resolve::<TestDependency>(),
            Ok(TestDependency {
                value: "Hello, world!".to_string()
            })
        );

        container.forget_scoped_instances();

        assert_eq!(container.resolve::<TestDependency>(), Err(Error::NotFound));
    }

    #[test]
    fn can_retrieve_a_registered_singleton() {
        let mut container = Container::new();

        container.singleton(&|_: &Container| TestDependency {
            value: "Hello, world!".to_string(),
        });

        let result = container.resolve::<TestDependency>().unwrap();

        assert_eq!(result.value, "Hello, world!");
    }

    #[test]
    fn returns_singleton_over_binding() {
        let mut container = Container::new();

        container.bind(|_: &Container| TestDependency {
            value: "Hello, world!".to_string(),
        });

        container.singleton(&|_: &Container| TestDependency {
            value: "Goodbye, world!".to_string(),
        });

        let result = container.resolve::<TestDependency>().unwrap();

        assert_eq!(result.value, "Goodbye, world!");
    }

    #[test]
    fn returns_error_when_not_found() {
        let container = Container::new();

        assert_eq!(container.resolve::<TestDependency>(), Err(Error::NotFound));
    }

    #[test]
    #[cfg(feature = "nightly")]
    fn can_resolve_a_binding_for_a_type_that_implements_default() {
        let container = Container::new();

        assert_eq!(container.resolve::<u64>(), Ok(u64::default()));
    }

    #[test]
    #[serial]
    fn can_use_global_container() {
        let container = Container::get_instance();

        let mut container_w = container.write().unwrap();
        container_w.bind(|_: &Container| TestDependency {
            value: "Hello, world!".to_string(),
        });
        drop(container_w);

        let container_r = container.read().unwrap();
        let result = container_r.resolve::<TestDependency>().unwrap();
        drop(container_r);

        assert_eq!(result.value, "Hello, world!");
    }
}
