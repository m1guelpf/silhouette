# Silhouette

> A simple dependency injection library for Rust

[![crates.io](https://img.shields.io/crates/v/silhouette.svg)](https://crates.io/crates/silhouette)
[![download count badge](https://img.shields.io/crates/d/silhouette.svg)](https://crates.io/crates/silhouette)
[![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/silhouette)

## About Silhouette

Silhouette implements a simple service container in Rust for dependency injection. It not only provides a `Container` struct for local usage, but also a static interface (under `facade::Container`) for easily managing dependencies throughout your application. It's heavily inspired by [Laravel's Service Container](https://laravel.com/docs/container).

## Getting Started

```rust
use silhouette::facade::Container;

struct DBPool {}
struct DBConnection {}

// will always use the same pool
Container::singleton(&|_| DBPool::new())?;

// will resolve a new connection each time
Container::bind(&|container| -> DBConnection {
    let shared_pool = container.resolve::<DBPool>().unwrap();

    shared_pool.get_conn()
})?;

// somewhere else in your app...
let connection: DBConnection = Container::resolve()?;
```

Refer to the [documentation on docs.rs](https://docs.rs/silhouette) for detailed usage instructions.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
