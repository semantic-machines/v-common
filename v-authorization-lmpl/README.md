# v-authorization-lmpl

LMDB implementation for Veda authorization system.

## Description

This crate provides LMDB-based storage backend for the Veda authorization framework. It implements the `AuthorizationContext` trait from `v_authorization` crate using Lightning Memory-Mapped Database (LMDB).

## Features

- High-performance authorization data storage using LMDB
- Optional caching layer for improved performance
- Statistics collection support
- Thread-safe read operations

## Usage

```rust
use v_authorization_lmpl::LmdbAzContext;

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

## Configuration

The context can be configured with:
- `max_read_counter`: Number of authorization operations before database reconnection
- `stat_collector_url`: Optional URL for statistics collection
- `stat_mode`: Statistics collection mode ("full", "minimal", or "none")
- `use_cache`: Enable/disable authorization cache

## Dependencies

- `lmdb-rs-m`: LMDB Rust bindings
- `v_authorization`: Core authorization framework
- `nng`: Nanomsg-next-generation for statistics reporting

