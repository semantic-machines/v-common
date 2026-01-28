# Implementation Details

## Database Backends

### LMDB (via heed)

#### Environment Setup

```rust
const DB_PATH: &str = "./data/acl-indexes/";
const CACHE_DB_PATH: &str = "./data/acl-cache-indexes/";

// Open LMDB environment
let env = unsafe { 
    EnvOpenOptions::new()
        .max_dbs(1)
        .open(DB_PATH)
}?;
```

**Configuration:**
- `max_dbs(1)`: Single unnamed database
- `unsafe`: Required by heed API (memory-mapped I/O)

#### Transaction Management

```rust
// Read-only transaction
let txn = env.read_txn()?;

// Open database
let db: Database<Str, Str> = env.open_database(&txn, None)?;

// Query data
let value = db.get(&txn, key)?;
```

**Features:**
- Zero-copy reads via memory mapping
- MVCC (Multi-Version Concurrency Control)
- Snapshot isolation
- Copy-on-write semantics

#### Storage Implementation

```rust
pub struct AzLmdbStorage<'a> {
    txn: &'a RoTxn<'a>,              // Read transaction
    db: Database<Str, Str>,           // Main database
    cache_txn: Option<&'a RoTxn<'a>>, // Cache transaction
    cache_db: Option<Database<Str, Str>>, // Cache database
    stat: &'a mut Option<Stat>,       // Statistics
}
```

**Query Strategy:**
1. Try cache (if available)
2. Fall back to main database
3. Record statistics

### libmdbx

#### Database Setup

```rust
const DB_PATH: &str = "./data/acl-mdbx-indexes/";
const CACHE_DB_PATH: &str = "./data/acl-cache-mdbx-indexes/";

// Open MDBX database
let db = Database::<NoWriteMap>::open(&path)?;
```

**Configuration:**
- `NoWriteMap`: Read-only access mode
- Automatic geometry management
- Improved error handling

#### Transaction Management

```rust
// Begin read-only transaction
let txn = db.begin_ro_txn()?;

// Open table
let table = txn.open_table(None)?;

// Query data
let value = txn.get::<Vec<u8>>(&table, key.as_bytes())?;

// Convert to string
let value_str = std::str::from_utf8(&value)?;
```

**Differences from LMDB:**
- Table-based API (vs database-based)
- Explicit transaction begin/commit
- Byte array values (requires UTF-8 conversion)
- Better error diagnostics

#### Storage Implementation

```rust
pub struct AzMdbxStorage<'a> {
    db: &'a Database<NoWriteMap>,      // Main database
    cache_db: Option<&'a Database<NoWriteMap>>, // Cache database
    stat: &'a mut Option<Stat>,        // Statistics
}
```

**Helper Method:**
```rust
fn read_from_db(
    &mut self,
    db: &Database<NoWriteMap>,
    key: &str,
    from_cache: bool
) -> io::Result<Option<String>>
```

This reduces code duplication for cache and main database access.

## Authorization Logic

### Helper Trait Implementation

The `AuthorizationHelper` trait provides common authorization logic:

```rust
fn authorize_and_trace_impl(
    &mut self,
    uri: &str,
    user_uri: &str,
    request_access: u8,
    is_check_for_reload: bool,
    trace: &mut Trace,
) -> Result<u8, std::io::Error> {
    // Increment and check counter
    let counter = self.get_authorize_counter() + 1;
    self.set_authorize_counter(counter);
    
    if counter >= self.get_max_authorize_counter() {
        self.set_authorize_counter(0);
        // Next call will reconnect
    }

    // Try authorization
    match self.authorize_use_db(uri, user_uri, request_access, is_check_for_reload, trace) {
        Ok(r) => return Ok(r),
        Err(_e) => {
            // Retry on error
            info!("retrying authorization after error");
        }
    }
    
    // Second attempt
    self.authorize_use_db(uri, user_uri, request_access, is_check_for_reload, trace)
}
```

**Key Points:**
- Counter incremented on each call
- Automatic retry on first error
- Counter reset when threshold reached

### Statistics Integration

```rust
pub(crate) fn authorize_with_stat<T: AuthorizationHelper>(
    ctx: &mut T,
    uri: &str,
    user_uri: &str,
    request_access: u8,
    is_check_for_reload: bool,
) -> Result<u8, std::io::Error> {
    let start_time = SystemTime::now();

    // Perform authorization
    let r = ctx.authorize_and_trace_impl(uri, user_uri, request_access, is_check_for_reload, &mut t);

    // Record statistics
    if let Some(stat) = ctx.get_stat_mut() {
        if stat.mode == StatMode::Full || stat.mode == StatMode::Minimal {
            let elapsed = start_time.elapsed().unwrap_or_default();
            stat.point.set_duration(elapsed);
            if let Err(e) = stat.point.flush() {
                warn!("fail flush stat, err={:?}", e);
            }
        }
    }

    r
}
```

**Timing:**
- Start: Before authorization
- End: After authorization
- Only recorded if statistics enabled

### Macro Implementation

The `impl_authorization_context!` macro generates the `AuthorizationContext` trait implementation:

```rust
#[macro_export]
macro_rules! impl_authorization_context {
    ($type:ty) => {
        impl v_authorization::common::AuthorizationContext for $type {
            fn authorize(
                &mut self,
                uri: &str,
                user_uri: &str,
                request_access: u8,
                is_check_for_reload: bool,
            ) -> Result<u8, std::io::Error> {
                $crate::common::authorize_with_stat(
                    self, uri, user_uri, request_access, is_check_for_reload
                )
            }

            fn authorize_and_trace(
                &mut self,
                uri: &str,
                user_uri: &str,
                request_access: u8,
                is_check_for_reload: bool,
                trace: &mut v_authorization::common::Trace,
            ) -> Result<u8, std::io::Error> {
                self.authorize_and_trace_impl(
                    uri, user_uri, request_access, is_check_for_reload, trace
                )
            }
        }
    };
}
```

**Usage:**
```rust
impl_authorization_context!(LmdbAzContext);
impl_authorization_context!(MdbxAzContext);
```

This eliminates code duplication between backends.

## Statistics System

### Message Collection

```rust
pub(crate) fn collect(&mut self, message: String) {
    self.message_buffer.push_back(message);
}
```

Messages are buffered and sent in batches to reduce network overhead.

### Message Format

```rust
pub(crate) fn format_stat_message(
    key: &str,
    use_cache: bool,
    from_cache: bool
) -> String {
    match (use_cache, from_cache) {
        (true, true) => format!("{}/C", key),      // From cache
        (true, false) => format!("{}/cB", key),    // Cache miss
        (false, _) => format!("{}/B", key),        // No cache
    }
}
```

**Format Examples:**
- `"d:user:123/C"`: User data from cache
- `"d:doc:456/cB"`: Document data, cache miss
- `"d:group:789/B"`: Group data, no cache

### Message Publishing

```rust
pub(crate) fn flush(&mut self) -> Result<(), nng::Error> {
    if !self.is_connected {
        self.connect()?;
    }

    // Combine messages
    let combined_message = self.message_buffer
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<&str>>()
        .join(";");

    // Format with metadata
    let message_with_timestamp = format!(
        "{},{},{}",
        self.sender_id,
        self.duration.as_micros(),
        combined_message
    );

    // Send via nanomsg
    self.socket.send(message_with_timestamp.as_bytes())?;

    // Clear buffer
    self.message_buffer.clear();

    Ok(())
}
```

**Message Structure:**
```
sender_id,duration_us,key1/flag;key2/flag;...
```

**Example:**
```
abc12345,1234,d:user:123/C;d:doc:456/cB;d:group:789/B
```

### Connection Management

```rust
pub(crate) fn new(url: &str) -> Result<Self, nng::Error> {
    let socket = Socket::new(Protocol::Pub0)?;
    let sender_id: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    
    Ok(Self {
        socket,
        url: url.to_string(),
        is_connected: false,
        message_buffer: VecDeque::new(),
        sender_id,
        duration: Duration::default(),
    })
}

fn connect(&mut self) -> Result<(), nng::Error> {
    self.socket.dial(&self.url)?;
    self.is_connected = true;
    Ok(())
}
```

**Features:**
- Lazy connection (on first flush)
- Unique sender ID for tracking
- Pub/Sub pattern via nanomsg

## Global State Management

### Lazy Initialization

```rust
// LMDB
static GLOBAL_ENV: LazyLock<Mutex<Option<Arc<Env>>>> = 
    LazyLock::new(|| Mutex::new(None));

// MDBX
static GLOBAL_DB: LazyLock<Mutex<Option<Arc<Database<NoWriteMap>>>>> = 
    LazyLock::new(|| Mutex::new(None));
```

**Benefits:**
- Thread-safe initialization
- Shared across all contexts
- Lazy allocation

### Environment Access Pattern

```rust
let env = {
    let mut env_lock = GLOBAL_ENV.lock().unwrap();
    
    if let Some(existing_env) = env_lock.as_ref() {
        // Reuse existing
        existing_env.clone()
    } else {
        // Create new
        let new_env = open_database()?;
        let arc_env = Arc::new(new_env);
        *env_lock = Some(arc_env.clone());
        arc_env
    }
};
```

**Pattern:**
1. Lock global state
2. Check if already initialized
3. If yes: clone Arc reference
4. If no: create, store, and clone
5. Release lock

### Reset for Testing

```rust
pub fn reset_global_envs() {
    let mut env = GLOBAL_ENV.lock().unwrap();
    *env = None;
    
    let mut cache_env = GLOBAL_CACHE_ENV.lock().unwrap();
    *cache_env = None;
    
    info!("Reset global environments");
}
```

**Usage:**
```rust
#[test]
fn test_something() {
    // Test code...
    
    // Cleanup
    reset_lmdb_global_envs();
}
```

### Synchronization

```rust
pub fn sync_env() -> bool {
    let env_opt = GLOBAL_ENV.lock().unwrap();
    if let Some(env) = env_opt.as_ref() {
        match env.force_sync() {
            Ok(_) => {
                info!("Successfully synced environment");
                true
            },
            Err(e) => {
                error!("Failed to sync: {:?}", e);
                false
            }
        }
    } else {
        true // No environment to sync
    }
}
```

Forces all pending writes to disk. Useful before:
- Tests
- Shutdown
- Backup operations

## Storage Trait Implementation

### LMDB Storage

```rust
impl<'a> Storage for AzLmdbStorage<'a> {
    fn get(&mut self, key: &str) -> io::Result<Option<String>> {
        // Try cache first
        if let Some(cache_db) = self.cache_db {
            if let Some(cache_txn) = self.cache_txn {
                match cache_db.get(cache_txn, key) {
                    Ok(Some(val)) => {
                        // Record cache hit
                        if let Some(stat) = self.stat {
                            if stat.mode == StatMode::Full {
                                stat.point.collect(
                                    format_stat_message(key, true, true)
                                );
                            }
                        }
                        return Ok(Some(val.to_string()));
                    },
                    Ok(None) | Err(_) => {
                        // Continue to main database
                    }
                }
            }
        }

        // Try main database
        match self.db.get(self.txn, key) {
            Ok(Some(val)) => {
                // Record main database hit
                if let Some(stat) = self.stat {
                    if stat.mode == StatMode::Full {
                        stat.point.collect(
                            format_stat_message(key, self.cache_db.is_some(), false)
                        );
                    }
                }
                Ok(Some(val.to_string()))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(Error::other(format!("db.get {:?}, {}", e, key))),
        }
    }
    
    // Other methods delegate to v_authorization::record_formats
    fn decode_rec_to_rights(&self, src: &str, result: &mut Vec<ACLRecord>) 
        -> (bool, Option<DateTime<Utc>>) 
    {
        decode_rec_to_rights(src, result)
    }
    
    // ... similar for other methods
}
```

### MDBX Storage

```rust
impl<'a> AzMdbxStorage<'a> {
    fn read_from_db(
        &mut self,
        db: &Database<NoWriteMap>,
        key: &str,
        from_cache: bool
    ) -> io::Result<Option<String>> {
        let txn = db.begin_ro_txn()
            .map_err(|e| Error::other(format!("failed to begin transaction {:?}", e)))?;
        
        let table = txn.open_table(None)
            .map_err(|e| Error::other(format!("failed to open table {:?}", e)))?;
        
        match txn.get::<Vec<u8>>(&table, key.as_bytes()) {
            Ok(Some(val)) => {
                let val_str = std::str::from_utf8(&val)
                    .map_err(|_| Error::other("Failed to decode UTF-8"))?;
                
                // Record statistics
                if let Some(stat) = self.stat {
                    if stat.mode == StatMode::Full {
                        stat.point.collect(
                            format_stat_message(key, self.cache_db.is_some(), from_cache)
                        );
                    }
                }
                
                Ok(Some(val_str.to_string()))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(Error::other(format!("db.get {:?}, {}", e, key))),
        }
    }
}

impl<'a> Storage for AzMdbxStorage<'a> {
    fn get(&mut self, key: &str) -> io::Result<Option<String>> {
        // Try cache
        if let Some(cache_db) = self.cache_db {
            if let Ok(Some(value)) = self.read_from_db(cache_db, key, true) {
                return Ok(Some(value));
            }
        }

        // Try main database
        self.read_from_db(self.db, key, false)
    }
    
    // ... other methods similar to LMDB
}
```

**Key Difference:**
MDBX requires UTF-8 conversion because it works with byte arrays, while LMDB (via heed) works directly with strings.

## Error Handling Patterns

### Database Opening

```rust
loop {
    let path = PathBuf::from(DB_PATH);
    
    if !path.exists() {
        error!("Database not found at: {}", path.display());
        thread::sleep(Duration::from_secs(3));
        continue;
    }
    
    match open_database(&path) {
        Ok(db) => {
            info!("Opened database at: {}", DB_PATH);
            break db;
        },
        Err(e) => {
            error!("Error opening database: {:?}. Retrying...", e);
            thread::sleep(Duration::from_secs(3));
        }
    }
}
```

**Rationale:**
- Database may not exist yet during startup
- Filesystem may be temporarily unavailable
- Prevents application crash on startup

### Query Errors

```rust
match db.get(&txn, key) {
    Ok(Some(val)) => Ok(Some(val.to_string())),
    Ok(None) => Ok(None),  // Not an error, just not found
    Err(e) => Err(Error::other(format!("db.get {:?}, {}", e, key))),
}
```

**Distinction:**
- `Ok(None)`: Key not found (normal)
- `Err(...)`: Database error (retry-able)

### Statistics Errors

```rust
if let Err(e) = stat.point.flush() {
    warn!("fail flush stat, err={:?}", e);
}
```

Statistics failures are logged but don't affect authorization results.

