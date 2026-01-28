# Architecture

## System Overview

The `v-authorization-impl` library provides database storage backends for the Veda authorization system. It implements the `AuthorizationContext` trait from `v_authorization` crate and offers two database engine options.

```
┌─────────────────────────────────────────────────────────────┐
│                   Application Layer                         │
├─────────────────────────────────────────────────────────────┤
│              v_authorization::AuthorizationContext          │
├─────────────────────────────────────────────────────────────┤
│                     v-authorization-impl                     │
│  ┌──────────────┐         ┌──────────────┐                 │
│  │  AzContext   │────────▶│  AzDbType    │                 │
│  └──────┬───────┘         └──────────────┘                 │
│         │                                                    │
│    ┌────┴────┐                                              │
│    │         │                                              │
│    ▼         ▼                                              │
│ ┌─────┐  ┌─────┐                                           │
│ │LMDB │  │MDBX │                                           │
│ └─────┘  └─────┘                                           │
├─────────────────────────────────────────────────────────────┤
│              Database Files (LMDB / libmdbx)                │
└─────────────────────────────────────────────────────────────┘
```

## Component Architecture

### Core Components

#### 1. Unified Context (`AzContext`)

The `AzContext` enum provides a unified interface for both database backends:

```rust
pub enum AzContext {
    Lmdb(LmdbAzContext),
    Mdbx(MdbxAzContext),
}
```

**Responsibilities:**
- Runtime selection of database backend
- Forwarding authorization requests to appropriate implementation
- Providing consistent API across backends

#### 2. LMDB Context (`LmdbAzContext`)

LMDB-based implementation using the `heed` library.

**Key Features:**
- Stable, well-tested backend
- ACID compliance
- Memory-mapped file access
- Copy-on-write snapshots

**Structure:**
```rust
pub struct LmdbAzContext {
    env: Arc<Env>,                    // Shared LMDB environment
    cache_env: Option<Arc<Env>>,      // Optional cache environment
    authorize_counter: u64,            // Operation counter
    max_authorize_counter: u64,        // Reconnection threshold
    stat: Option<Stat>,                // Statistics collector
}
```

#### 3. MDBX Context (`MdbxAzContext`)

libmdbx-based implementation with modern features.

**Key Features:**
- Modern LMDB fork
- Improved performance
- Better error handling
- More efficient memory usage

**Structure:**
```rust
pub struct MdbxAzContext {
    db: Arc<Database<NoWriteMap>>,    // Shared MDBX database
    cache_db: Option<Arc<Database<NoWriteMap>>>, // Optional cache
    authorize_counter: u64,
    max_authorize_counter: u64,
    stat: Option<Stat>,
}
```

### Helper Traits

#### AuthorizationHelper

Internal trait providing common functionality for authorization contexts.

```rust
pub(crate) trait AuthorizationHelper {
    fn get_stat_mut(&mut self) -> &mut Option<Stat>;
    fn get_authorize_counter(&self) -> u64;
    fn get_max_authorize_counter(&self) -> u64;
    fn set_authorize_counter(&mut self, value: u64);
    fn authorize_use_db(...) -> Result<u8, std::io::Error>;
    fn authorize_and_trace_impl(...) -> Result<u8, std::io::Error>;
}
```

**Responsibilities:**
- Counter management
- Database access abstraction
- Retry logic
- Statistics integration

#### Storage

Trait from `v_authorization` for database operations.

```rust
pub trait Storage {
    fn get(&mut self, key: &str) -> io::Result<Option<String>>;
    fn fiber_yield(&self);
    fn decode_rec_to_rights(...);
    fn decode_rec_to_rightset(...);
    fn decode_filter(...);
}
```

**Implementations:**
- `AzLmdbStorage`: LMDB storage implementation
- `AzMdbxStorage`: libmdbx storage implementation

### Statistics Subsystem

#### StatPub

Handles statistics collection and publishing.

```rust
pub(crate) struct StatPub {
    socket: Socket,              // nanomsg socket
    url: String,                 // Collector URL
    is_connected: bool,          // Connection state
    message_buffer: VecDeque<String>, // Message queue
    sender_id: String,           // Unique sender ID
    duration: Duration,          // Operation duration
}
```

**Features:**
- Buffered message collection
- Automatic connection management
- Batch message sending
- Unique sender identification

#### StatMode

Enumeration of statistics collection levels.

```rust
pub(crate) enum StatMode {
    Full,     // Collect all details
    Minimal,  // Only timing
    None,     // Disabled
}
```

## Data Flow

### Authorization Request Flow

```
1. Application calls authorize()
   │
   ├─▶ AzContext::authorize()
   │   │
   │   ├─▶ authorize_with_stat()
   │       │
   │       ├─▶ Start timing
   │       │
   │       ├─▶ AuthorizationHelper::authorize_and_trace_impl()
   │       │   │
   │       │   ├─▶ Increment counter
   │       │   │
   │       │   ├─▶ authorize_use_db()
   │       │       │
   │       │       ├─▶ Create transaction
   │       │       │
   │       │       ├─▶ Open database(s)
   │       │       │
   │       │       ├─▶ Create Storage instance
   │       │       │
   │       │       ├─▶ Call v_authorization::authorize()
   │       │           │
   │       │           ├─▶ Storage::get() (possibly multiple times)
   │       │           │   │
   │       │           │   ├─▶ Try cache (if enabled)
   │       │           │   │
   │       │           │   └─▶ Try main database
   │       │           │
   │       │           └─▶ Return access rights
   │       │
   │       ├─▶ Collect statistics (if enabled)
   │       │
   │       └─▶ Return result
   │
   └─▶ Return to application
```

### Database Access Flow

#### With Cache Enabled

```
Storage::get(key)
   │
   ├─▶ Open cache transaction
   │   │
   │   ├─▶ Query cache database
   │   │
   │   ├─▶ Cache HIT?
   │   │   │
   │   │   ├─▶ YES: Return cached value
   │   │   │         Record stat: key/C
   │   │   │
   │   │   └─▶ NO: Continue to main database
   │
   ├─▶ Open main transaction
   │   │
   │   ├─▶ Query main database
   │   │
   │   └─▶ Return value
   │         Record stat: key/cB (cache enabled, from DB)
   │
   └─▶ Return result
```

#### Without Cache

```
Storage::get(key)
   │
   ├─▶ Open main transaction
   │   │
   │   ├─▶ Query main database
   │   │
   │   └─▶ Return value
   │         Record stat: key/B (from DB)
   │
   └─▶ Return result
```

## Thread Safety

### Global Shared Environments

Both backends use global shared database environments:

```rust
// LMDB
static GLOBAL_ENV: LazyLock<Mutex<Option<Arc<Env>>>> = ...;
static GLOBAL_CACHE_ENV: LazyLock<Mutex<Option<Arc<Env>>>> = ...;

// MDBX
static GLOBAL_DB: LazyLock<Mutex<Option<Arc<Database<NoWriteMap>>>>> = ...;
static GLOBAL_CACHE_DB: LazyLock<Mutex<Option<Arc<Database<NoWriteMap>>>>> = ...;
```

**Design Rationale:**
- Reduce memory overhead
- Share read-only snapshots
- Avoid repeated database opening
- Enable efficient multi-threaded access

**Thread Safety Guarantees:**
- Database environments are shared via `Arc`
- Initialization is protected by `Mutex`
- Read transactions are thread-safe
- Each thread can have its own context

### Context Lifecycle

```
Thread A                  Thread B                  Global State
   │                         │                          │
   ├─ new()                  │                          │
   │  │                      │                          │
   │  └─▶ Lock GLOBAL_ENV    │                          │
   │      │                  │                          │
   │      ├─ First access    │                          │
   │      │  Create Env ────────────────────▶ Arc<Env>  │
   │      │                  │                  │        │
   │      └─ Clone Arc ◀──────────────────────┘         │
   │                         │                           │
   │                         ├─ new()                    │
   │                         │  │                        │
   │                         │  └─▶ Lock GLOBAL_ENV      │
   │                         │      │                    │
   │                         │      └─ Clone Arc ◀───────┘
   │                         │                           
   ├─ authorize()            ├─ authorize()             
   │  (independent)          │  (independent)           
   │                         │                          
```

## Performance Considerations

### Counter Management

The `max_authorize_counter` mechanism prevents potential issues:

1. **Memory Leaks**: Long-running read transactions can prevent garbage collection
2. **Stale Data**: Periodic reconnection ensures fresh snapshots
3. **Resource Limits**: Limits transaction lifetime

**Typical Values:**
- Low-traffic: 1000 - 10000
- Medium-traffic: 10000 - 50000
- High-traffic: 50000 - 100000

### Caching Strategy

The optional cache layer improves performance for frequently accessed data:

**Benefits:**
- Reduced latency for hot data
- Lower load on main database
- Better throughput under high concurrency

**Tradeoffs:**
- Additional disk space
- Potential stale reads
- Cache coherency complexity

### Statistics Overhead

Statistics collection impacts performance:

- **None**: No overhead
- **Minimal**: ~1-2% overhead (timing only)
- **Full**: ~5-10% overhead (detailed tracking)

## Error Handling

### Retry Logic

The library implements automatic retry for transient errors:

```rust
match self.authorize_use_db(...) {
    Ok(r) => return Ok(r),
    Err(_e) => {
        // Log and retry once
        info!("retrying authorization after error");
    }
}
self.authorize_use_db(...) // Second attempt
```

### Database Initialization

Robust initialization with automatic retry:

```rust
loop {
    if !path.exists() {
        error!("Database not found");
        thread::sleep(Duration::from_secs(3));
        continue;
    }
    
    match open_database(&path) {
        Ok(db) => break db,
        Err(e) => {
            error!("Error opening database: {:?}", e);
            thread::sleep(Duration::from_secs(3));
        }
    }
}
```

This ensures the system can recover from temporary issues like:
- Database not yet created
- File system not mounted
- Temporary permission issues
- Lock contention

## Extension Points

### Adding New Backends

To add a new database backend:

1. Create new context struct (e.g., `RocksDbAzContext`)
2. Implement `AuthorizationHelper` trait
3. Create storage implementation
4. Add variant to `AzDbType` enum
5. Update `AzContext` to include new backend

### Custom Statistics

To implement custom statistics:

1. Replace `StatPub` with your implementation
2. Implement similar interface for collecting metrics
3. Update `Stat` struct to use new publisher
4. Modify `format_stat_message` for your format

