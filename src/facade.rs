use std::sync::RwLock;

/// Static interface for the container.
pub struct Container {}

impl Container {
    fn get_instance() -> &'static RwLock<crate::Container> {
        crate::Container::get_instance()
    }

    /// Register a binding with the container.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to get write access to the container.
    pub fn bind<T: 'static>(
        factory: impl Fn(&crate::Container) -> T + 'static + Sync + Send,
    ) -> Result<(), Error> {
        let container = Self::get_instance();

        let mut container_w = container.write().map_err(|_| Error::Lock)?;
        container_w.bind(factory);
        drop(container_w);

        Ok(())
    }

    /// Register a binding if it hasn't already been registered.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to get write access to the container.
    pub fn bind_if<T: 'static>(
        factory: impl Fn(&crate::Container) -> T + 'static + Sync + Send,
    ) -> Result<(), Error> {
        let container = Self::get_instance();

        let mut container_w = container.write().map_err(|_| Error::Lock)?;
        container_w.bind_if(factory);
        drop(container_w);

        Ok(())
    }

    /// Register a scoped binding in the container.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to get write access to the container.
    pub fn scoped<T: 'static + Clone + Send + Sync>(
        factory: &(impl Fn(&crate::Container) -> T + 'static),
    ) -> Result<(), Error> {
        let container = Self::get_instance();

        let mut container_w = container.write().map_err(|_| Error::Lock)?;
        container_w.scoped(factory);
        drop(container_w);

        Ok(())
    }

    /// Register a scoped binding if it hasn't already been registered.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to get write access to the container.
    pub fn scoped_if<T: 'static + Clone + Send + Sync>(
        factory: &(impl Fn(&crate::Container) -> T + 'static),
    ) -> Result<(), Error> {
        let container = Self::get_instance();

        let mut container_w = container.write().map_err(|_| Error::Lock)?;
        container_w.scoped_if(factory);
        drop(container_w);

        Ok(())
    }

    /// Register a shared binding in the container.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to get write access to the container.
    pub fn singleton<T: 'static + Clone + Send + Sync>(
        factory: &(impl Fn(&crate::Container) -> T + 'static),
    ) -> Result<(), Error> {
        let container = Self::get_instance();

        let mut container_w = container.write().map_err(|_| Error::Lock)?;
        container_w.singleton(factory);
        drop(container_w);

        Ok(())
    }

    /// Register a shared binding if it hasn't already been registered.
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to get write access to the container.
    pub fn singleton_if<T: 'static + Clone + Send + Sync>(
        factory: &(impl Fn(&crate::Container) -> T + 'static),
    ) -> Result<(), Error> {
        let container = Self::get_instance();

        let mut container_w = container.write().map_err(|_| Error::Lock)?;
        container_w.singleton_if(factory);
        drop(container_w);

        Ok(())
    }

    /// Resolve the given type from the container.
    ///
    /// # Errors
    ///
    /// Returns an error if it fails to get read access to the container, if the requested type cannot be found, or if the requested type cannot be cast from the binding.
    pub fn resolve<T: 'static>() -> Result<T, Error> {
        let container = Self::get_instance();

        let container_r = container.read().map_err(|_| Error::Lock)?;
        Ok(container_r.resolve()?)
    }

    /// Clear all of the scoped instances from the container.
    ///
    /// # Errors
    ///
    /// Returns an error if it fails to get write access to the container.
    pub fn forget_scoped_instances() -> Result<(), Error> {
        let container = Self::get_instance();

        let mut container_w = container.write().map_err(|_| Error::Lock)?;
        container_w.forget_scoped_instances();
        drop(container_w);

        Ok(())
    }

    /// Flush the container of all bindings and resolved instances.
    ///
    /// # Errors
    ///
    /// Returns an error if it fails to get write access to the container.
    pub fn flush() -> Result<(), Error> {
        let container = Self::get_instance();

        let mut container_w = container.write().map_err(|_| Error::Lock)?;
        container_w.flush();
        drop(container_w);

        Ok(())
    }
}

/// Possible errors that can occur when interacting with the container's static interface.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Failed to get container instance.
    #[error("Failed to get container instance")]
    Lock,

    /// Container error.
    #[error(transparent)]
    Container(#[from] crate::Error),
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
    #[serial]
    fn can_register_a_binding() {
        Container::bind(|_| TestDependency {
            value: "Hello, world!".to_string(),
        })
        .unwrap();
    }

    #[test]
    #[serial]
    fn can_register_a_singleton() {
        Container::singleton(&|_| TestDependency {
            value: "Hello, world!".to_string(),
        })
        .unwrap();
    }

    #[test]
    #[serial]
    fn can_register_a_scoped_binding() {
        Container::scoped(&|_| TestDependency {
            value: "Hello, world!".to_string(),
        })
        .unwrap();
    }

    #[test]
    #[serial]
    fn registering_a_binding_clears_previous_singleton() {
        Container::singleton(&|_| TestDependency {
            value: "Hello, world!".to_string(),
        })
        .unwrap();

        Container::bind(|_| TestDependency {
            value: "Goodbye, world!".to_string(),
        })
        .unwrap();

        assert_eq!(
            Container::resolve::<TestDependency>(),
            Ok(TestDependency {
                value: "Goodbye, world!".to_string()
            })
        );
    }

    #[test]
    #[serial]
    fn can_retrieve_a_registered_binding() {
        Container::bind(|_| TestDependency {
            value: "Hello, world!".to_string(),
        })
        .unwrap();

        let result = Container::resolve::<TestDependency>().unwrap();

        assert_eq!(result.value, "Hello, world!");
    }

    #[test]
    #[serial]
    fn can_retrieve_a_registered_scoped_binding_until_flush() {
        #[derive(Debug, Clone, PartialEq)]
        struct FlushableDependency {
            value: String,
        }

        Container::scoped(&|_| FlushableDependency {
            value: "Hello, world!".to_string(),
        })
        .unwrap();

        assert_eq!(
            Container::resolve::<FlushableDependency>(),
            Ok(FlushableDependency {
                value: "Hello, world!".to_string()
            })
        );

        Container::forget_scoped_instances().unwrap();

        assert_eq!(
            Container::resolve::<FlushableDependency>(),
            Err(Error::Container(crate::Error::NotFound))
        );
    }

    #[test]
    #[serial]
    fn can_retrieve_a_registered_singleton() {
        Container::singleton(&|_| TestDependency {
            value: "Hello, world!".to_string(),
        })
        .unwrap();

        let result = Container::resolve::<TestDependency>().unwrap();

        assert_eq!(result.value, "Hello, world!");
    }

    #[test]
    #[serial]
    fn returns_singleton_over_binding() {
        Container::bind(|_| TestDependency {
            value: "Hello, world!".to_string(),
        })
        .unwrap();

        Container::singleton(&|_| TestDependency {
            value: "Goodbye, world!".to_string(),
        })
        .unwrap();

        let result = Container::resolve::<TestDependency>().unwrap();

        assert_eq!(result.value, "Goodbye, world!");
    }

    #[test]
    #[serial]
    fn returns_error_when_not_found() {
        assert_eq!(
            Container::resolve::<std::fs::File>().unwrap_err(),
            Error::Container(crate::Error::NotFound)
        );
    }

    #[test]
    #[serial]
    #[cfg(feature = "nightly")]
    fn can_resolve_a_binding_for_a_type_that_implements_default() {
        assert_eq!(Container::resolve::<u64>(), Ok(u64::default()));
    }
}
