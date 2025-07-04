# v-storage Usage Examples

This folder contains usage examples of the `v-storage` library for the Veda platform. Each example demonstrates different aspects and capabilities of the library.

## Examples Overview

### 1. 📚 [basic_usage.rs](basic_usage.rs) - Basic Usage
Demonstrates basic storage operations:
- ✅ Creating storage through Builder pattern
- ✅ Basic operations: put, get, remove, count
- ✅ Working with different data types (strings, binary data)
- ✅ Using all StorageId types (Individuals, Tickets, Az)
- ✅ Error handling
- ✅ Backward compatibility demonstration

```bash
cargo run --example basic_usage
```

### 2. 🏗️ [storage_types.rs](storage_types.rs) - Storage Types Comparison
Shows differences between architectural approaches:
- 🎭 **VStorage** - dynamic storage (trait objects)
- 🔧 **VStorageGeneric** - generic storage (compile-time typing)
- ⚡ **VStorageEnum** - enum storage (static dispatch)
- 🔄 API consistency between types
- 💡 Selection recommendations

```bash
cargo run --example storage_types
```

### 3. 🏭 [factory_patterns.rs](factory_patterns.rs) - Factory Patterns
Demonstrates various ways to create storages:
- 🏗️ **Builder Pattern** - step-by-step creation with validation
- 🏭 **Provider Pattern** - ready factory methods
- ⚙️ **Config Pattern** - creation from configuration structures
- 🎯 **Generic Methods** - statically typed builders
- 📊 Approach comparison and recommendations

```bash
cargo run --example factory_patterns
```

### 4. 👤 [individual_operations.rs](individual_operations.rs) - Working with Individual
Shows working with Individual objects from the Veda platform:
- 📋 Different Individual data formats (JSON)
- ⚠️ Error handling and parsing situations
- 🗂️ Individual in different storage types (StorageId)
- 📦 Batch operations with Individual
- 💡 Best practices and recommendations

```bash
cargo run --example individual_operations
```



## How to Run Examples

### All examples at once
```bash
# Run all examples in sequence
cargo run --example basic_usage
cargo run --example storage_types  
cargo run --example factory_patterns
cargo run --example individual_operations
```

### With additional features
```bash
# With Tarantool support
cargo run --features tt_2 --example factory_patterns

# With old Tokio version support
cargo run --features tokio_0_2 --example factory_patterns

# With all features
cargo run --features "tt_2 tokio_0_2" --example individual_operations
```



## Architectural Patterns

### 🎯 Strategy Pattern
```rust
// Unified Storage interface for all storage types
let mut storage: Box<dyn Storage> = /* any implementation */;
storage.put_value(StorageId::Individuals, "key", "value");
```

### 🏗️ Builder Pattern
```rust
let storage = VStorage::builder()
    .memory()
    .build()?;
```

### 🏭 Abstract Factory
```rust
// Creation through Provider
let storage = StorageProvider::memory();

// Creation through Config
let storage = VStorage::from_config(StorageConfig::Memory)?;
```

### 🔧 Generic Containers
```rust
// Compile-time typing without vtable overhead
let storage = VMemoryStorage::new(MemoryStorage::new());
```

## Storage Types

| Type | Description | Dispatch | Flexibility | Usage |
|------|-------------|----------|-------------|-------|
| **VStorage** | Dynamic dispatch through trait objects | Runtime (vtable) | Maximum | Runtime type determination |
| **VStorageGeneric** | Compile-time typing | Static | Medium | Known type at compile time |
| **VStorageEnum** | Static dispatch through enum | Static | Limited | Static dispatch preferred |

## StorageId Types

- **Individuals** - main Individual objects of the Veda platform
- **Tickets** - system tickets/tasks
- **Az** - authorization and permission data

## Possible Errors and Solutions

### Compilation Errors

**Features not found:**
```bash
# Install required features
cargo check --features "tt_2 tokio_0_2"
```

**Missing Individual types:**
```bash
# Make sure v_individual_model is in dependencies
cargo check --example individual_operations
```

### Runtime Errors

**LMDB unavailable:**
- LMDB storage requires system libraries
- Use MemoryStorage for testing

**Tarantool unavailable:**
- TTStorage requires a running Tarantool server
- Use feature flag `tt_2` or `tt_3`

**Remote unavailable:**
- RemoteStorage requires network connection
- Check address and server availability

## Additional Resources

- [API Documentation](../src/lib.rs)
- [Tests](../tests/) - additional usage examples
- [Cargo.toml](../Cargo.toml) - features configuration

## Contributing

When adding new examples:
1. Create a file in the `examples/` folder
2. Add documentation at the beginning of the file
3. Update this README
4. Make sure the example compiles with different features
5. Add the example to CI/CD if necessary

---

**💡 Tip:** Start with `basic_usage.rs` to understand the basics, then study `storage_types.rs` to choose the right architectural approach. 