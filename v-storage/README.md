# v-storage

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Build Status](https://img.shields.io/badge/build-passing-green.svg)](https://github.com/your-org/v-storage)

A flexible storage abstraction library for the Veda platform. Provides unified interface for different storage backends including memory, LMDB, Tarantool, and remote storage.

## 🚀 Features

- **Multiple Storage Backends**: Memory, LMDB, Tarantool (TTStorage), Remote storage
- **Unified API**: Common `Storage` trait for all backends
- **Three Architecture Patterns**: Dynamic dispatch, generic containers, enum dispatch
- **Individual Support**: Native support for Veda platform Individual objects
- **Zero-cost Abstractions**: Compile-time optimization options with minimal overhead
- **Factory Patterns**: Builder, Provider, and Config patterns for easy construction
- **Error Handling**: Comprehensive error types and result handling
- **Backward Compatibility**: Support for legacy API methods

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
v-storage = "0.1.0"

# Optional features
v-storage = { version = "0.1.0", features = ["tt_2", "tokio_0_2"] }
```

### Available Features

- `tt_2` - Tarantool 2.x support
- `tt_3` - Tarantool 3.x support  
- `tokio_0_2` - Tokio 0.2 runtime support
- `tokio_1` - Tokio 1.x runtime support

## 🏃 Quick Start

```rust
use v_storage::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create memory storage
    let storage = VStorage::builder()
        .memory()
        .build()?;
    let mut storage = VStorage::new(storage);

    // Store Individual with semantic predicates
    storage.put_value(StorageId::Individuals, "user:1", r#"{
        "@": "user:1",
        "rdf:type": [{"type": "Uri", "data": "foaf:Person"}],
        "rdfs:label": [{"type": "String", "data": "John Smith - Software Engineer"}],
        "foaf:name": [{"type": "String", "data": "John"}],
        "foaf:familyName": [{"type": "String", "data": "Smith"}],
        "foaf:age": [{"type": "Integer", "data": 30}],
        "foaf:mbox": [{"type": "String", "data": "john.smith@example.com"}],
        "veda:hasPosition": [{"type": "Uri", "data": "position:software_engineer"}],
        "org:memberOf": [{"type": "Uri", "data": "org:development_team"}]
    }"#)?;
    
    // Retrieve Individual
    if let StorageResult::Ok(data) = storage.get_value(StorageId::Individuals, "user:1") {
        println!("User Individual: {} bytes", data.len());
    }

    Ok(())
}
```

## 🏗️ Architecture

### Storage Types

| Type | Description | Dispatch | Use Case |
|------|-------------|----------|----------|
| **VStorage** | Dynamic dispatch with trait objects | Runtime | Maximum flexibility, runtime type selection |
| **VStorageGeneric** | Compile-time typed containers | Static | Type safety, known storage type |
| **VStorageEnum** | Static dispatch through enum | Static | Applications preferring enum dispatch |

### Storage Backends

- **Memory Storage** - In-memory HashMap-based storage
- **LMDB Storage** - Lightning Memory-Mapped Database  
- **Tarantool Storage** - In-memory NoSQL database
- **Remote Storage** - Network-based storage client

### StorageId Types

- **Individuals** - Main Individual objects for Veda platform
- **Tickets** - System tickets and tasks
- **Az** - Authorization and permission data

## 📖 Usage Examples

### Basic Operations

```rust
use v_storage::*;

// Create storage using Builder pattern
let storage = VStorage::builder()
    .memory()
    .build()?;
let mut storage = VStorage::new(storage);

// Basic operations with semantic Individual data
storage.put_value(StorageId::Individuals, "person:example", r#"{
    "@": "person:example",
    "rdf:type": [{"type": "Uri", "data": "foaf:Person"}],
    "rdfs:label": [{"type": "String", "data": "Example Person"}],
    "foaf:name": [{"type": "String", "data": "Example"}]
}"#)?;
storage.get_value(StorageId::Individuals, "person:example")?;
storage.remove_value(StorageId::Individuals, "person:example")?;
storage.count(StorageId::Individuals)?;
```

### Factory Patterns

```rust
// Builder Pattern
let storage = VStorage::builder()
    .lmdb("/path/to/db", StorageMode::ReadWrite, None)
    .build()?;

// Provider Pattern  
let storage = StorageProvider::memory();

// Config Pattern
let config = StorageConfig::Memory;
let storage = VStorage::from_config(config)?;
```

### Working with Individual Objects

```rust
use v_individual_model::onto::individual::Individual;

let mut individual = Individual::default();
let result = storage.get_individual(
    StorageId::Individuals, 
    "person:john", 
    &mut individual
);

match result {
    v_result_code::ResultCode::Ok => {
        println!("Individual loaded: {}", individual.get_id());
    },
    v_result_code::ResultCode::NotFound => {
        println!("Individual not found");
    },
    _ => println!("Error loading individual"),
}
```

### Static Dispatch Usage

```rust
// Use VStorageEnum for static dispatch
let mut storage = VStorageEnum::memory();

// Batch operations with semantic Individual data
for i in 0..1000 {
    let individual_data = format!(r#"{{
        "@": "person:{}",
        "rdf:type": [{{"type": "Uri", "data": "foaf:Person"}}],
        "rdfs:label": [{{"type": "String", "data": "Person {}"}}],
        "foaf:name": [{{"type": "String", "data": "Person{}"}}],
        "veda:index": [{{"type": "Integer", "data": {}}}]
    }}"#, i, i, i, i);
    
    storage.put_value(
        StorageId::Individuals, 
        &format!("person:{}", i), 
        &individual_data
    )?;
}
```

## 🎯 Design Patterns

### Strategy Pattern
Unified `Storage` interface allows switching between different storage implementations:

```rust
fn process_data(storage: &mut dyn Storage) {
    // Works with any storage implementation
    storage.put_value(StorageId::Individuals, "person:demo", r#"{
        "@": "person:demo",
        "rdf:type": [{"type": "Uri", "data": "foaf:Person"}],
        "rdfs:label": [{"type": "String", "data": "Demo Person"}]
    }"#);
}
```

### Builder Pattern
Fluent API for configuring storage:

```rust
let storage = VStorage::builder()
    .memory()
    .build()?;
```

### Abstract Factory
Multiple ways to create storage instances:

```rust
// Through provider
let storage1 = StorageProvider::memory();

// Through config
let storage2 = VStorage::from_config(StorageConfig::Memory)?;

// Through builder
let storage3 = VStorage::builder().memory().build()?;
```

## 📊 Architecture Comparison

Theoretical dispatch overhead (in practice, database I/O dominates):

| Storage Type | Dispatch | Memory | Flexibility |
|--------------|----------|--------|-------------|
| VStorageEnum | Static (enum match) | Stack | Fixed set of types |
| VStorageGeneric | Static (monomorphization) | Direct | Compile-time known |
| VStorage | Dynamic (vtable) | Heap | Runtime selection |

*Note: In real applications, storage backend (LMDB, Tarantool, network) dominates timing.*

## 🔧 Configuration

### Memory Storage
```rust
let storage = VStorage::builder()
    .memory()
    .build()?;
```

### LMDB Storage
```rust
let storage = VStorage::builder()
    .lmdb("/path/to/database", StorageMode::ReadWrite, Some(1000))
    .build()?;
```

### Tarantool Storage
```rust
// Requires tt_2 or tt_3 feature
let storage = VStorage::builder()
    .tarantool("127.0.0.1:3301", "username", "password")
    .build()?;
```

### Remote Storage
```rust
let storage = VStorage::builder()
    .remote("127.0.0.1:8080")
    .build()?;
```

## 📚 Examples

The [`examples/`](examples/) directory contains comprehensive examples:

- **[basic_usage.rs](examples/basic_usage.rs)** - Basic operations and API usage
- **[storage_types.rs](examples/storage_types.rs)** - Comparison of different storage types
- **[factory_patterns.rs](examples/factory_patterns.rs)** - Various construction patterns
- **[individual_operations.rs](examples/individual_operations.rs)** - Working with Individual objects

Run examples:
```bash
cargo run --example basic_usage
cargo run --example storage_types
cargo run --example factory_patterns
cargo run --example individual_operations
```

## 🛠️ Development

### Building

```bash
# Basic build
cargo build

# With all features
cargo build --features "tt_2 tokio_0_2"

# Release build
cargo build --release
```

### Testing

```bash
# Run tests
cargo test

# Run tests with features
cargo test --features "tt_2 tokio_0_2"

# Run integration tests
cargo test --test integration_tests
```

### Documentation

```bash
# Generate documentation
cargo doc --open

# Generate with examples
cargo doc --examples --open
```

## 🔍 Error Handling

The library uses comprehensive error types:

```rust
match storage.get_value(StorageId::Individuals, "key") {
    StorageResult::Ok(value) => println!("Value: {}", value),
    StorageResult::NotFound => println!("Key not found"),
    StorageResult::Error(e) => println!("Error: {}", e),
}
```

## 🧪 Testing

```bash
# Unit tests
cargo test

# Integration tests  
cargo test --test integration_tests

# Example tests
cargo test --examples

# All tests with release optimization
cargo test --release
```

## 📋 Requirements

- **Rust**: 1.70 or higher
- **Operating System**: Linux, macOS, Windows
- **Dependencies**: See [Cargo.toml](Cargo.toml)

### Optional Requirements

- **LMDB**: System LMDB libraries for LMDBStorage
- **Tarantool**: Running Tarantool instance for TTStorage
- **Network**: Connectivity for RemoteStorage

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

1. Clone the repository
2. Install Rust 1.70+
3. Run `cargo build`
4. Run `cargo test`
5. Check examples with `cargo run --example basic_usage`

### Code Style

- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Add tests for new features
- Update documentation

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Related Projects

- **[Veda Platform](https://github.com/semantic-machines/veda)** - Semantic data management platform
- **[v-individual-model](https://crates.io/crates/v-individual-model)** - Individual object model
- **[LMDB](https://symas.com/lmdb/)** - Lightning Memory-Mapped Database
- **[Tarantool](https://www.tarantool.io/)** - In-memory database and application server

## 💡 FAQ

**Q: Which storage type should I choose?**
A: Use `VStorageEnum` for applications preferring static dispatch, `VStorageGeneric` for type safety with known storage types, and `VStorage` for maximum flexibility.

**Q: Can I switch storage backends at runtime?**
A: Yes, with `VStorage` dynamic dispatch. Generic types require compile-time selection.

**Q: Is the library thread-safe?**
A: Storage instances are not thread-safe by default. Use appropriate synchronization for concurrent access.

**Q: How do I handle network failures with RemoteStorage?**
A: The library returns `StorageResult::Error` for network issues. Implement retry logic in your application.

---

**Built with ❤️ for the Veda platform** 