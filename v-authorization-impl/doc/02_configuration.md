# Configuration

## Configuration Options

The authorization context can be configured with several parameters:

### Basic Configuration

```rust
let mut az_ctx = AzContext::new(db_type, max_read_counter);
```

- `db_type`: `AzDbType::Lmdb` or `AzDbType::Mdbx`
- `max_read_counter`: Number of operations before database reconnection

### Advanced Configuration

```rust
let mut az_ctx = AzContext::new_with_config(
    db_type,
    max_read_counter,
    stat_collector_url,
    stat_mode,
    use_cache
);
```

## Parameters

### max_read_counter

Type: `u64`

Defines the number of authorization operations before the database connection is refreshed. This helps prevent potential memory leaks or stale connections.

**Recommendations:**
- Production systems: 10000 - 100000
- High-load systems: 50000 - 100000
- Development/testing: 1000
- Never reconnect: `u64::MAX`

Example:
```rust
// Reconnect after 50000 operations
let mut az_ctx = LmdbAzContext::new(50000);
```

### stat_collector_url

Type: `Option<String>`

URL of the statistics collector endpoint. If provided, the context will send performance statistics to this endpoint using the nanomsg protocol.

**Format:** `"tcp://hostname:port"`

Example:
```rust
let mut az_ctx = AzContext::new_with_config(
    AzDbType::Lmdb,
    10000,
    Some("tcp://localhost:9999".to_string()),
    Some("full".to_string()),
    None
);
```

### stat_mode

Type: `Option<String>`

Controls the level of statistics collection.

**Valid values:**
- `"full"`: Collect detailed statistics including individual key access
- `"minimal"`: Collect only timing statistics
- `"none"` or `"off"`: Disable statistics collection

**Default:** `"none"`

Example:
```rust
let mut az_ctx = AzContext::new_with_config(
    AzDbType::Mdbx,
    10000,
    Some("tcp://localhost:9999".to_string()),
    Some("minimal".to_string()),
    None
);
```

### use_cache

Type: `Option<bool>`

Enables or disables the authorization cache layer. When enabled, frequently accessed authorization data is stored in a separate cache database for faster retrieval.

**Default:** `false`

**Cache Benefits:**
- Faster authorization checks for frequently accessed resources
- Reduced load on main database
- Improved response times under high load

**Cache Considerations:**
- Requires additional disk space
- May serve stale data if not properly invalidated
- Cache database must exist at the configured path

Example:
```rust
let mut az_ctx = AzContext::new_with_config(
    AzDbType::Lmdb,
    10000,
    None,
    None,
    Some(true) // Enable cache
);
```

## Database Paths

### LMDB Paths

- Main database: `./data/acl-indexes/`
- Cache database: `./data/acl-cache-indexes/`

### libmdbx Paths

- Main database: `./data/acl-mdbx-indexes/`
- Cache database: `./data/acl-cache-mdbx-indexes/`

**Note:** These paths are hardcoded in the implementation to prevent accidental mixing of different backend formats.

## Statistics Message Format

When statistics collection is enabled, messages are sent in the following format:

```
sender_id,duration_microseconds,key1/flag;key2/flag;...
```

Where flags are:
- `/C`: Data retrieved from cache
- `/cB`: Cache enabled but data from main database
- `/B`: Cache disabled, data from main database

## Environment Setup

Before using the library, ensure the database directories exist:

```bash
# For LMDB
mkdir -p ./data/acl-indexes
mkdir -p ./data/acl-cache-indexes

# For libmdbx
mkdir -p ./data/acl-mdbx-indexes
mkdir -p ./data/acl-cache-mdbx-indexes
```

The databases must be pre-populated with authorization data by the Veda system or compatible tools.

## Example: Production Configuration

```rust
use v_authorization_impl::{AzContext, AzDbType};

fn create_production_context() -> AzContext {
    AzContext::new_with_config(
        AzDbType::Mdbx,           // Use modern libmdbx
        50000,                     // Reconnect every 50k operations
        Some("tcp://stats.example.com:9999".to_string()),
        Some("minimal".to_string()), // Collect timing stats
        Some(true)                 // Enable cache
    )
}
```

## Example: Development Configuration

```rust
use v_authorization_impl::{AzContext, AzDbType};

fn create_dev_context() -> AzContext {
    AzContext::new_with_config(
        AzDbType::Lmdb,           // Use stable LMDB
        1000,                      // Reconnect frequently for testing
        None,                      // No statistics
        None,                      // No stats mode
        Some(false)                // Disable cache for testing
    )
}
```

