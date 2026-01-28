# Troubleshooting

## Common Issues

### Database Connection Errors

#### Problem: "Database does not exist at path"

**Symptoms:**
```
ERROR: Database does not exist at path: ./data/acl-indexes/data.mdb
Retrying database connection...
```

**Solutions:**

1. Check if database directory exists:
```bash
ls -la ./data/acl-indexes/
```

2. Create missing directories:
```bash
mkdir -p ./data/acl-indexes
mkdir -p ./data/acl-mdbx-indexes
```

3. Verify database files:
```bash
# LMDB should have data.mdb and lock.mdb
ls ./data/acl-indexes/

# MDBX should have mdbx.dat and mdbx.lck
ls ./data/acl-mdbx-indexes/
```

4. Check file permissions:
```bash
chmod -R 755 ./data/
```

#### Problem: "Error opening environment/database"

**Possible Causes:**
- Database corruption
- Insufficient permissions
- Another process holding lock
- Disk full

**Solutions:**

1. Check disk space:
```bash
df -h ./data/
```

2. Check file permissions:
```bash
ls -l ./data/acl-indexes/
```

3. Check for stale locks:
```bash
# LMDB
rm ./data/acl-indexes/lock.mdb

# MDBX
rm ./data/acl-mdbx-indexes/mdbx.lck
```

4. Check for other processes:
```bash
lsof ./data/acl-indexes/data.mdb
```

### Authorization Errors

#### Problem: Always returning "Access denied"

**Debugging Steps:**

1. Enable trace to see authorization chain:
```rust
let mut acl = String::new();
let mut group = String::new();
let mut info = String::new();

let mut trace = Trace {
    acl: &mut acl,
    is_acl: true,
    group: &mut group,
    is_group: true,
    info: &mut info,
    is_info: true,
    str_num: 0,
};

let result = ctx.authorize_and_trace(
    uri, user_uri, request_access, false, &mut trace
)?;

println!("ACL chain: {}", acl);
println!("Groups: {}", group);
```

2. Verify URIs format:
```rust
// Correct format
"d:organization:doc123"
"d:user:alice"

// Incorrect (missing prefix)
"organization:doc123"
"user:alice"
```

3. Check database content:
```bash
# For LMDB (requires mdb_dump tool)
mdb_dump -p ./data/acl-indexes/

# For MDBX (requires mdbx_dump tool)
mdbx_dump -p ./data/acl-mdbx-indexes/
```

#### Problem: Intermittent authorization failures

**Possible Causes:**
- Database transaction timeout
- Concurrent write operations
- Counter reset during reconnection

**Solutions:**

1. Increase max_read_counter:
```rust
// Instead of
let mut ctx = AzContext::new(AzDbType::Lmdb, 1000);

// Use
let mut ctx = AzContext::new(AzDbType::Lmdb, 100000);
```

2. Add retry logic:
```rust
let mut attempts = 0;
let max_attempts = 3;

loop {
    match ctx.authorize(uri, user_uri, access, false) {
        Ok(result) => break Ok(result),
        Err(e) => {
            attempts += 1;
            if attempts >= max_attempts {
                break Err(e);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
```

### Cache Issues

#### Problem: Cache not improving performance

**Diagnostic Steps:**

1. Verify cache database exists:
```bash
ls -la ./data/acl-cache-indexes/
ls -la ./data/acl-cache-mdbx-indexes/
```

2. Check cache is enabled:
```rust
let mut ctx = AzContext::new_with_config(
    AzDbType::Lmdb,
    10000,
    None,
    None,
    Some(true)  // Must be true
);
```

3. Enable statistics to verify cache hits:
```rust
let mut ctx = AzContext::new_with_config(
    AzDbType::Lmdb,
    10000,
    Some("tcp://localhost:9999".to_string()),
    Some("full".to_string()),
    Some(true)
);
```

4. Check cache size:
```bash
du -h ./data/acl-cache-indexes/
```

#### Problem: Stale data from cache

**Solution:**
Cache data may be outdated if the main database was updated but cache wasn't. The system doesn't automatically invalidate cache.

**Workaround:**
1. Clear cache database periodically
2. Disable cache if data freshness is critical
3. Implement external cache invalidation mechanism

### Statistics Issues

#### Problem: Statistics not being collected

**Debugging Steps:**

1. Verify statistics configuration:
```rust
let mut ctx = AzContext::new_with_config(
    AzDbType::Lmdb,
    10000,
    Some("tcp://localhost:9999".to_string()),  // Must provide URL
    Some("full".to_string()),                   // Must not be "none"
    None
);
```

2. Check statistics collector is running:
```bash
# Check if collector is listening
netstat -an | grep 9999

# Or using ss
ss -tuln | grep 9999
```

3. Enable logging to see statistics:
```bash
RUST_LOG=info cargo run
```

4. Test nanomsg connection:
```rust
use nng::{Socket, Protocol};

let socket = Socket::new(Protocol::Pub0)?;
socket.dial("tcp://localhost:9999")?;
socket.send(b"test message")?;
```

#### Problem: Statistics collector connection fails

**Solutions:**

1. Make collector URL optional:
```rust
let mut ctx = AzContext::new_with_config(
    AzDbType::Lmdb,
    10000,
    None,  // Disable stats if collector unavailable
    None,
    None
);
```

2. Check network connectivity:
```bash
telnet localhost 9999
```

3. Verify nanomsg protocol support:
```bash
# Install nanomsg tools
apt-get install nanomsg-utils

# Test pub/sub
nanocat --pub --bind tcp://localhost:9999
nanocat --sub --connect tcp://localhost:9999
```

### Performance Issues

#### Problem: Slow authorization checks

**Diagnostic Steps:**

1. Measure authorization time:
```rust
use std::time::Instant;

let start = Instant::now();
let result = ctx.authorize(uri, user_uri, access, false)?;
let elapsed = start.elapsed();

if elapsed.as_millis() > 10 {
    println!("Slow authorization: {:?}", elapsed);
}
```

2. Check database size:
```bash
du -h ./data/acl-indexes/
du -h ./data/acl-mdbx-indexes/
```

3. Monitor system resources:
```bash
# CPU usage
top -p $(pgrep -f your_app)

# Memory usage
ps aux | grep your_app

# I/O wait
iostat -x 1
```

**Solutions:**

1. Enable caching:
```rust
let mut ctx = AzContext::new_with_config(
    db_type,
    10000,
    None,
    None,
    Some(true)  // Enable cache
);
```

2. Switch to MDBX (often faster):
```rust
let mut ctx = AzContext::new(AzDbType::Mdbx, 10000);
```

3. Optimize database:
```bash
# Compact LMDB
mdb_copy -c ./data/acl-indexes ./data/acl-indexes-compact
mv ./data/acl-indexes ./data/acl-indexes-old
mv ./data/acl-indexes-compact ./data/acl-indexes

# Compact MDBX
mdbx_copy -c ./data/acl-mdbx-indexes ./data/acl-mdbx-indexes-compact
```

4. Increase max_read_counter to reduce reconnections:
```rust
let mut ctx = AzContext::new(AzDbType::Lmdb, u64::MAX);
```

#### Problem: High memory usage

**Possible Causes:**
- Large database mapped into memory
- Memory leak in application
- Too many contexts created

**Solutions:**

1. Monitor memory:
```bash
# Check resident set size
ps aux | grep your_app | awk '{print $6}'

# Detailed memory map
pmap -x $(pgrep -f your_app)
```

2. Use single shared context:
```rust
// Instead of creating many contexts
let ctx = Arc::new(Mutex::new(AzContext::new(AzDbType::Lmdb, 10000)));

// Share across threads
let ctx_clone = Arc::clone(&ctx);
```

3. Reset environments periodically (testing only):
```rust
drop(ctx);
sync_lmdb_env();
reset_lmdb_global_envs();
```

### Multi-threading Issues

#### Problem: Deadlocks or hangs with shared context

**Symptoms:**
```
Thread 1: Holding lock on context
Thread 2: Waiting for lock (hangs)
```

**Solutions:**

1. Use finer-grained locking:
```rust
// Bad: Hold lock during entire operation
let mut ctx = shared_ctx.lock().unwrap();
let result = ctx.authorize(...);
process_result(result);

// Good: Release lock quickly
let result = {
    let mut ctx = shared_ctx.lock().unwrap();
    ctx.authorize(...)
};
process_result(result);
```

2. Create context per thread:
```rust
thread::spawn(move || {
    // Each thread has own context
    // They share underlying database
    let mut ctx = AzContext::new(AzDbType::Lmdb, 10000);
    ctx.authorize(...)
});
```

#### Problem: "Resource temporarily unavailable" error

**Cause:** Too many concurrent read transactions

**Solutions:**

1. Limit concurrent operations:
```rust
use std::sync::Semaphore;

let semaphore = Arc::new(Semaphore::new(10)); // Max 10 concurrent

let permit = semaphore.acquire().await;
let result = ctx.authorize(...);
drop(permit);
```

2. Reduce max_read_counter:
```rust
let mut ctx = AzContext::new(AzDbType::Lmdb, 1000);
```

### Compilation Issues

#### Problem: Linking errors with LMDB or MDBX

**Solutions:**

1. Install system dependencies:
```bash
# Ubuntu/Debian
apt-get install liblmdb-dev

# Fedora/RHEL
yum install lmdb-devel

# macOS
brew install lmdb
```

2. For MDBX, ensure Rust toolchain is up to date:
```bash
rustup update
```

#### Problem: Version conflicts

**Solution:**
Check `Cargo.toml` and ensure compatible versions:
```toml
[dependencies]
heed = "0.22.0"
libmdbx = "0.6.3"
v_authorization = "=0.5.1"
```

### Testing Issues

#### Problem: Tests fail due to shared state

**Solution:**
Use test isolation:
```rust
#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_something() {
        let _guard = TEST_LOCK.lock().unwrap();
        
        // Test code
        
        // Cleanup
        reset_lmdb_global_envs();
    }
}
```

Or run serially:
```bash
cargo test -- --test-threads=1
```

## Logging and Debugging

### Enable Detailed Logging

```bash
# All logs
RUST_LOG=debug cargo run

# Library-specific logs
RUST_LOG=v_authorization_impl=debug cargo run

# Multiple modules
RUST_LOG=v_authorization_impl=debug,v_authorization=info cargo run
```

### Log Configuration in Code

```rust
use env_logger;

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    
    // Your code
}
```

### Debug Database State

#### LMDB

```bash
# Dump all keys
mdb_dump -p ./data/acl-indexes/

# Count entries
mdb_stat ./data/acl-indexes/

# Check specific key
mdb_dump -p -s d:user:alice ./data/acl-indexes/
```

#### MDBX

```bash
# Dump all keys
mdbx_dump -p ./data/acl-mdbx-indexes/

# Statistics
mdbx_stat ./data/acl-mdbx-indexes/

# Check integrity
mdbx_chk ./data/acl-mdbx-indexes/
```

## Getting Help

### Information to Provide

When reporting issues, include:

1. **Version information:**
```bash
cargo --version
rustc --version
cat Cargo.toml | grep v-authorization-impl
```

2. **Environment:**
```bash
uname -a
df -h ./data/
```

3. **Minimal reproduction:**
```rust
use v_authorization_impl::{AzContext, AzDbType};

fn main() {
    let mut ctx = AzContext::new(AzDbType::Lmdb, 10000);
    let result = ctx.authorize("d:test:r", "d:test:u", 1, false);
    println!("Result: {:?}", result);
}
```

4. **Logs:**
```bash
RUST_LOG=debug cargo run 2>&1 | tee debug.log
```

5. **Database state:**
```bash
mdb_stat ./data/acl-indexes/
du -h ./data/
```

