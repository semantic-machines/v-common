# v-authorization-impl

LMDB and MDBX implementation for Veda authorization system.

## Description

This crate provides LMDB-based and libmdbx-based storage backends for the Veda authorization framework. It implements the `AuthorizationContext` trait from `v_authorization` crate using two different database engines:
- **LMDB** (via heed) - stable, well-tested
- **libmdbx** - modern fork of LMDB with improvements

## Features

- High-performance authorization data storage
- Support for two database backends (LMDB and libmdbx)
- Optional caching layer for improved performance
- Statistics collection support
- Thread-safe read operations
- Separate database paths to prevent data corruption

## Database Paths

To prevent accidental database corruption, different backends use different paths:
- **LMDB**: `./data/acl-indexes/` (main), `./data/acl-cache-indexes/` (cache)
- **libmdbx**: `./data/acl-mdbx-indexes/` (main), `./data/acl-cache-mdbx-indexes/` (cache)

## Usage

### Using LMDB backend (heed)

```rust
use v_authorization_impl::{LmdbAzContext, AzDbType, AzContext};

// Create with default settings
let mut az_ctx = LmdbAzContext::default();

// Create with custom max read counter
let mut az_ctx = LmdbAzContext::new(10000);

// Create with full configuration
let mut az_ctx = LmdbAzContext::new_with_config(
    10000,
    Some("tcp://localhost:9999".to_string()),
    Some("full".to_string()),
    Some(true)
);
```

### Using libmdbx backend

```rust
use v_authorization_impl::{MdbxAzContext, AzDbType, AzContext};

// Create with default settings
let mut az_ctx = MdbxAzContext::default();

// Create with custom max read counter
let mut az_ctx = MdbxAzContext::new(10000);

// Create with full configuration
let mut az_ctx = MdbxAzContext::new_with_config(
    10000,
    Some("tcp://localhost:9999".to_string()),
    Some("full".to_string()),
    Some(true)
);
```

### Using unified context (recommended)

```rust
use v_authorization_impl::{AzContext, AzDbType};

// Choose database type at runtime
let db_type = AzDbType::Mdbx; // or AzDbType::Lmdb

// Create with default settings
let mut az_ctx = AzContext::new(db_type, 10000);

// Create with full configuration
let mut az_ctx = AzContext::new_with_config(
    db_type,
    10000,
    Some("tcp://localhost:9999".to_string()),
    Some("full".to_string()),
    Some(true)
);
```

## Configuration

The context can be configured with:
- `max_read_counter`: Number of authorization operations before database reconnection
- `stat_collector_url`: Optional URL for statistics collection
- `stat_mode`: Statistics collection mode ("full", "minimal", or "none")
- `use_cache`: Enable/disable authorization cache

## Dependencies

- `heed`: LMDB Rust bindings
- `libmdbx`: Modern LMDB fork
- `v_authorization`: Core authorization framework
- `nng`: Nanomsg-next-generation for statistics reporting


