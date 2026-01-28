# Getting Started

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
v-authorization-impl = "0.5.3"
```

## Dependencies

The library requires the following dependencies:

- `heed` (0.22.0): LMDB Rust bindings
- `libmdbx` (0.6.3): Modern LMDB fork
- `v_authorization` (0.5.1): Core authorization framework
- `nng` (1.0.1): Nanomsg-next-generation for statistics
- `log` (0.4): Logging facade
- `chrono` (0.4.41): Date and time handling
- `rand` (0.9.2): Random number generation for statistics

## Quick Start

### Using Unified Context (Recommended)

```rust
use v_authorization_impl::{AzContext, AzDbType};
use v_authorization::common::AuthorizationContext;

fn main() {
    // Choose database type
    let db_type = AzDbType::Mdbx; // or AzDbType::Lmdb
    
    // Create context with default settings
    let mut az_ctx = AzContext::new(db_type, 10000);
    
    // Perform authorization check
    let result = az_ctx.authorize(
        "d:resource_uri",
        "d:user_uri",
        1, // request_access (1 = Can Read)
        false
    );
    
    match result {
        Ok(access) => println!("Access granted: {}", access),
        Err(e) => eprintln!("Authorization error: {}", e),
    }
}
```

### Using LMDB Backend Directly

```rust
use v_authorization_impl::LmdbAzContext;
use v_authorization::common::AuthorizationContext;

fn main() {
    // Create LMDB context
    let mut az_ctx = LmdbAzContext::new(10000);
    
    // Use the context...
}
```

### Using libmdbx Backend Directly

```rust
use v_authorization_impl::MdbxAzContext;
use v_authorization::common::AuthorizationContext;

fn main() {
    // Create MDBX context
    let mut az_ctx = MdbxAzContext::new(10000);
    
    // Use the context...
}
```

## Basic Configuration

### Max Read Counter

The `max_read_counter` parameter defines how many authorization operations can be performed before the database connection is refreshed:

```rust
// Reconnect after every 10000 operations
let mut az_ctx = AzContext::new(AzDbType::Lmdb, 10000);

// Never reconnect (use u64::MAX)
let mut az_ctx = LmdbAzContext::default();
```

## Next Steps

- Read [Configuration](02_configuration.md) for advanced setup options
- See [API Reference](03_api_reference.md) for detailed method documentation
- Check [Examples](04_examples.md) for more usage patterns

