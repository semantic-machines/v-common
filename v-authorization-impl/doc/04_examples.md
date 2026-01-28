# Examples

## Basic Usage

### Simple Authorization Check

```rust
use v_authorization_impl::{AzContext, AzDbType};
use v_authorization::common::AuthorizationContext;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create context
    let mut ctx = AzContext::new(AzDbType::Lmdb, 10000);
    
    // Check if user can read document
    let access = ctx.authorize(
        "d:organization:doc123",
        "d:user:alice",
        1, // Request Read access
        false
    )?;
    
    if access & 1 != 0 {
        println!("User can read the document");
    } else {
        println!("Access denied");
    }
    
    Ok(())
}
```

### Multiple Access Rights

```rust
use v_authorization_impl::{LmdbAzContext};
use v_authorization::common::AuthorizationContext;

fn check_multiple_rights() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = LmdbAzContext::new(10000);
    
    // Request multiple rights: Read (1) + Update (4) + Delete (8)
    let requested = 1 | 4 | 8;
    
    let granted = ctx.authorize(
        "d:organization:doc456",
        "d:user:bob",
        requested,
        false
    )?;
    
    println!("Checking access rights for user Bob:");
    println!("  Can Read: {}", granted & 1 != 0);
    println!("  Can Create: {}", granted & 2 != 0);
    println!("  Can Update: {}", granted & 4 != 0);
    println!("  Can Delete: {}", granted & 8 != 0);
    
    Ok(())
}
```

## Advanced Usage

### With Statistics Collection

```rust
use v_authorization_impl::{AzContext, AzDbType};
use v_authorization::common::AuthorizationContext;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create context with statistics enabled
    let mut ctx = AzContext::new_with_config(
        AzDbType::Mdbx,
        10000,
        Some("tcp://localhost:9999".to_string()),
        Some("full".to_string()),
        None
    );
    
    // Perform authorization (stats will be collected)
    let access = ctx.authorize(
        "d:project:task789",
        "d:user:charlie",
        1 | 4, // Read + Update
        false
    )?;
    
    println!("Access granted: {}", access);
    
    Ok(())
}
```

### With Caching

```rust
use v_authorization_impl::{MdbxAzContext};
use v_authorization::common::AuthorizationContext;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create context with cache enabled
    let mut ctx = MdbxAzContext::new_with_config(
        10000,
        None,
        None,
        Some(true) // Enable cache
    );
    
    // First access - will read from main database
    let access1 = ctx.authorize(
        "d:cached:resource",
        "d:user:alice",
        1,
        false
    )?;
    
    // Subsequent accesses may be served from cache
    let access2 = ctx.authorize(
        "d:cached:resource",
        "d:user:alice",
        1,
        false
    )?;
    
    assert_eq!(access1, access2);
    
    Ok(())
}
```

### With Trace Information

```rust
use v_authorization_impl::{LmdbAzContext};
use v_authorization::common::{AuthorizationContext, Trace};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = LmdbAzContext::new(10000);
    
    // Prepare trace storage
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
    
    // Authorize with trace
    let access = ctx.authorize_and_trace(
        "d:document:report",
        "d:user:dave",
        1,
        false,
        &mut trace
    )?;
    
    // Print trace information
    println!("Access: {}", access);
    println!("ACL chain: {}", acl);
    println!("Groups: {}", group);
    println!("Additional info: {}", info);
    
    Ok(())
}
```

## Multi-threaded Usage

### Shared Context Across Threads

```rust
use v_authorization_impl::{AzContext, AzDbType};
use v_authorization::common::AuthorizationContext;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create shared context
    let ctx = Arc::new(Mutex::new(
        AzContext::new(AzDbType::Lmdb, 10000)
    ));
    
    let mut handles = vec![];
    
    // Spawn multiple threads
    for i in 0..4 {
        let ctx_clone = Arc::clone(&ctx);
        let handle = thread::spawn(move || {
            let mut ctx = ctx_clone.lock().unwrap();
            
            // Each thread performs authorization
            match ctx.authorize(
                &format!("d:resource:{}", i),
                "d:user:alice",
                1,
                false
            ) {
                Ok(access) => println!("Thread {}: access = {}", i, access),
                Err(e) => eprintln!("Thread {}: error = {}", i, e),
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    Ok(())
}
```

### Multiple Contexts

```rust
use v_authorization_impl::{AzContext, AzDbType};
use v_authorization::common::AuthorizationContext;
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut handles = vec![];
    
    // Each thread creates its own context
    for i in 0..4 {
        let handle = thread::spawn(move || {
            // Contexts share the same underlying database
            let mut ctx = AzContext::new(AzDbType::Mdbx, 10000);
            
            match ctx.authorize(
                &format!("d:resource:{}", i),
                "d:user:alice",
                1,
                false
            ) {
                Ok(access) => println!("Thread {}: access = {}", i, access),
                Err(e) => eprintln!("Thread {}: error = {}", i, e),
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    Ok(())
}
```

## Testing Utilities

### Reset Environments for Tests

```rust
#[cfg(test)]
mod tests {
    use v_authorization_impl::{
        AzContext, AzDbType,
        reset_lmdb_global_envs,
        sync_lmdb_env
    };
    use v_authorization::common::AuthorizationContext;
    
    #[test]
    fn test_authorization() {
        // Create context
        let mut ctx = AzContext::new(AzDbType::Lmdb, 10000);
        
        // Perform test
        let result = ctx.authorize(
            "d:test:resource",
            "d:test:user",
            1,
            false
        );
        
        assert!(result.is_ok());
        
        // Sync before cleanup
        sync_lmdb_env();
        
        // Clean up after test
        drop(ctx);
        reset_lmdb_global_envs();
    }
}
```

## Runtime Backend Selection

### Configuration-based Backend

```rust
use v_authorization_impl::{AzContext, AzDbType};
use v_authorization::common::AuthorizationContext;

fn create_context_from_config(backend: &str) -> AzContext {
    let db_type = match backend {
        "lmdb" => AzDbType::Lmdb,
        "mdbx" => AzDbType::Mdbx,
        _ => AzDbType::Lmdb, // Default
    };
    
    AzContext::new_with_config(
        db_type,
        10000,
        std::env::var("STAT_COLLECTOR_URL").ok(),
        std::env::var("STAT_MODE").ok(),
        Some(std::env::var("USE_CACHE")
            .map(|v| v == "true")
            .unwrap_or(false))
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read backend from environment or config
    let backend = std::env::var("AUTH_BACKEND")
        .unwrap_or_else(|_| "lmdb".to_string());
    
    let mut ctx = create_context_from_config(&backend);
    
    let access = ctx.authorize(
        "d:resource:example",
        "d:user:test",
        1,
        false
    )?;
    
    println!("Using {} backend, access: {}", backend, access);
    
    Ok(())
}
```

## Error Handling

### Comprehensive Error Handling

```rust
use v_authorization_impl::{LmdbAzContext};
use v_authorization::common::AuthorizationContext;
use std::io::ErrorKind;

fn check_access_with_retry(
    ctx: &mut LmdbAzContext,
    uri: &str,
    user_uri: &str,
    access: u8,
    max_retries: u32
) -> Result<u8, String> {
    let mut attempts = 0;
    
    loop {
        match ctx.authorize(uri, user_uri, access, false) {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempts += 1;
                if attempts >= max_retries {
                    return Err(format!("Failed after {} attempts: {}", attempts, e));
                }
                
                eprintln!("Authorization failed (attempt {}): {}", attempts, e);
                
                // Handle different error types
                match e.kind() {
                    ErrorKind::NotFound => {
                        return Err("Resource or user not found".to_string());
                    },
                    ErrorKind::PermissionDenied => {
                        return Err("Permission denied".to_string());
                    },
                    _ => {
                        // Retry on other errors
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = LmdbAzContext::new(10000);
    
    match check_access_with_retry(
        &mut ctx,
        "d:resource:important",
        "d:user:test",
        1,
        3
    ) {
        Ok(access) => println!("Access granted: {}", access),
        Err(e) => eprintln!("Authorization error: {}", e),
    }
    
    Ok(())
}
```

## Performance Monitoring

### Custom Statistics Handler

```rust
use v_authorization_impl::{AzContext, AzDbType};
use v_authorization::common::AuthorizationContext;
use std::time::Instant;

fn benchmark_authorization() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = AzContext::new(AzDbType::Mdbx, 10000);
    
    let iterations = 1000;
    let start = Instant::now();
    
    for i in 0..iterations {
        ctx.authorize(
            &format!("d:resource:{}", i % 100),
            "d:user:test",
            1,
            false
        )?;
    }
    
    let elapsed = start.elapsed();
    let avg_time = elapsed.as_micros() / iterations;
    
    println!("Total time: {:?}", elapsed);
    println!("Average time per check: {} μs", avg_time);
    println!("Operations per second: {}", 
             (iterations as f64 / elapsed.as_secs_f64()) as u64);
    
    Ok(())
}
```

