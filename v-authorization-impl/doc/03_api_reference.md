# API Reference

## Core Types

### AzDbType

Enum for selecting the database backend.

```rust
pub enum AzDbType {
    Lmdb,
    Mdbx,
}
```

**Variants:**
- `Lmdb`: Use LMDB backend (heed)
- `Mdbx`: Use libmdbx backend

### AzContext

Unified authorization context that wraps either LMDB or libmdbx implementation.

```rust
pub enum AzContext {
    Lmdb(LmdbAzContext),
    Mdbx(MdbxAzContext),
}
```

#### Methods

##### `new`

```rust
pub fn new(db_type: AzDbType, max_read_counter: u64) -> Self
```

Create a new authorization context with basic configuration.

**Parameters:**
- `db_type`: Database backend to use
- `max_read_counter`: Number of operations before reconnection

**Returns:** New `AzContext` instance

**Example:**
```rust
let ctx = AzContext::new(AzDbType::Lmdb, 10000);
```

##### `new_with_config`

```rust
pub fn new_with_config(
    db_type: AzDbType,
    max_read_counter: u64,
    stat_collector_url: Option<String>,
    stat_mode_str: Option<String>,
    use_cache: Option<bool>,
) -> Self
```

Create a new authorization context with full configuration.

**Parameters:**
- `db_type`: Database backend to use
- `max_read_counter`: Number of operations before reconnection
- `stat_collector_url`: Optional URL for statistics collector
- `stat_mode_str`: Statistics mode ("full", "minimal", "none")
- `use_cache`: Enable/disable cache layer

**Returns:** New `AzContext` instance

**Example:**
```rust
let ctx = AzContext::new_with_config(
    AzDbType::Mdbx,
    10000,
    Some("tcp://localhost:9999".to_string()),
    Some("full".to_string()),
    Some(true)
);
```

## Backend-Specific Types

### LmdbAzContext

LMDB-based authorization context.

```rust
pub struct LmdbAzContext { /* fields omitted */ }
```

#### Methods

##### `new`

```rust
pub fn new(max_read_counter: u64) -> LmdbAzContext
```

Create LMDB context with basic configuration.

##### `new_with_config`

```rust
pub fn new_with_config(
    max_read_counter: u64,
    stat_collector_url: Option<String>,
    stat_mode_str: Option<String>,
    use_cache: Option<bool>
) -> LmdbAzContext
```

Create LMDB context with full configuration.

##### `default`

```rust
impl Default for LmdbAzContext
```

Creates context with `max_read_counter = u64::MAX` (never reconnect).

### MdbxAzContext

libmdbx-based authorization context.

```rust
pub struct MdbxAzContext { /* fields omitted */ }
```

#### Methods

Same as `LmdbAzContext`:
- `new(max_read_counter: u64)`
- `new_with_config(...)`
- `impl Default`

## AuthorizationContext Trait

All context types implement the `AuthorizationContext` trait from `v_authorization::common`.

### Methods

#### `authorize`

```rust
fn authorize(
    &mut self,
    uri: &str,
    user_uri: &str,
    request_access: u8,
    is_check_for_reload: bool,
) -> Result<u8, std::io::Error>
```

Perform authorization check.

**Parameters:**
- `uri`: Resource URI to check access for
- `user_uri`: User URI requesting access
- `request_access`: Requested access rights (bitfield)
- `is_check_for_reload`: Whether to check if database needs reloading

**Returns:** 
- `Ok(u8)`: Granted access rights (bitfield)
- `Err(std::io::Error)`: Error during authorization

**Access Rights Bitfield:**
- `1` (0b00000001): Can Read
- `2` (0b00000010): Can Create
- `4` (0b00000100): Can Update
- `8` (0b00001000): Can Delete
- `16` (0b00010000): Can Append
- Multiple rights can be combined using bitwise OR

**Example:**
```rust
let result = ctx.authorize(
    "d:doc123",
    "d:user456",
    1 | 4, // Request Read and Update
    false
)?;

if result & 1 != 0 {
    println!("Can read");
}
if result & 4 != 0 {
    println!("Can update");
}
```

#### `authorize_and_trace`

```rust
fn authorize_and_trace(
    &mut self,
    uri: &str,
    user_uri: &str,
    request_access: u8,
    is_check_for_reload: bool,
    trace: &mut Trace,
) -> Result<u8, std::io::Error>
```

Perform authorization check with detailed trace information.

**Parameters:**
- Same as `authorize`, plus:
- `trace`: Mutable trace object to collect debug information

**Returns:** Same as `authorize`

**Example:**
```rust
use v_authorization::common::Trace;

let mut acl = String::new();
let mut group = String::new();
let mut info = String::new();

let mut trace = Trace {
    acl: &mut acl,
    is_acl: false,
    group: &mut group,
    is_group: false,
    info: &mut info,
    is_info: false,
    str_num: 0,
};

let result = ctx.authorize_and_trace(
    "d:doc123",
    "d:user456",
    1,
    false,
    &mut trace
)?;

println!("ACL used: {}", acl);
println!("Groups: {}", group);
println!("Additional info: {}", info);
```

## Utility Functions

### LMDB Utilities

#### `reset_lmdb_global_envs`

```rust
pub fn reset_lmdb_global_envs()
```

Reset global LMDB environments. Useful for testing.

**Warning:** Should only be called when no active contexts exist.

#### `sync_lmdb_env`

```rust
pub fn sync_lmdb_env() -> bool
```

Force synchronization of LMDB environment to disk.

**Returns:** `true` if successful, `false` otherwise

### libmdbx Utilities

#### `reset_mdbx_global_envs`

```rust
pub fn reset_mdbx_global_envs()
```

Reset global libmdbx databases. Useful for testing.

**Warning:** Should only be called when no active contexts exist.

#### `sync_mdbx_env`

```rust
pub fn sync_mdbx_env() -> bool
```

Force synchronization of libmdbx database to disk.

**Returns:** `true` if successful, `false` otherwise

## Re-exports

```rust
pub use v_authorization;
```

The library re-exports the `v_authorization` crate for convenience, providing access to:
- `v_authorization::common::AuthorizationContext`
- `v_authorization::common::Trace`
- `v_authorization::ACLRecord`
- `v_authorization::ACLRecordSet`
- And other types from the core authorization framework

## Thread Safety

All context types are designed to be used in multi-threaded environments:

- Database environments are shared globally across threads
- Read transactions are thread-safe
- Multiple contexts can be created in different threads
- Statistics collection is thread-safe

**Note:** Each context maintains its own counter, so `max_read_counter` applies per-context, not globally.

## Error Handling

Authorization methods return `Result<u8, std::io::Error>`. Common error scenarios:

1. **Database not found**: Database path doesn't exist or is inaccessible
2. **Transaction error**: Failed to create read transaction
3. **Data corruption**: Invalid data format in database
4. **UTF-8 decode error**: Database contains invalid UTF-8 data (libmdbx only)

The library includes automatic retry logic for transient database errors.

