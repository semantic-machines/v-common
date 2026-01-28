# Testing Guide

## Test Environment Setup

### Prerequisites

Before running tests, ensure the databases exist:

```bash
# Create database directories
mkdir -p ./data/acl-indexes
mkdir -p ./data/acl-cache-indexes
mkdir -p ./data/acl-mdbx-indexes
mkdir -p ./data/acl-cache-mdbx-indexes
```

### Test Data

Tests require pre-populated authorization databases. These are typically created by:
- Veda system initialization
- Migration scripts
- Test fixtures

## Unit Tests

### Testing LMDB Context

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use v_authorization::common::AuthorizationContext;

    #[test]
    fn test_lmdb_basic_authorization() {
        let mut ctx = LmdbAzContext::new(10000);
        
        let result = ctx.authorize(
            "d:test:resource",
            "d:test:user",
            1,
            false
        );
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_lmdb_with_cache() {
        let mut ctx = LmdbAzContext::new_with_config(
            10000,
            None,
            None,
            Some(true)
        );
        
        let result = ctx.authorize(
            "d:test:resource",
            "d:test:user",
            1,
            false
        );
        
        assert!(result.is_ok());
    }
}
```

### Testing MDBX Context

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use v_authorization::common::AuthorizationContext;

    #[test]
    fn test_mdbx_basic_authorization() {
        let mut ctx = MdbxAzContext::new(10000);
        
        let result = ctx.authorize(
            "d:test:resource",
            "d:test:user",
            1,
            false
        );
        
        assert!(result.is_ok());
    }
}
```

### Testing Unified Context

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use v_authorization::common::AuthorizationContext;

    #[test]
    fn test_unified_context_lmdb() {
        let mut ctx = AzContext::new(AzDbType::Lmdb, 10000);
        
        let result = ctx.authorize(
            "d:test:resource",
            "d:test:user",
            1,
            false
        );
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_unified_context_mdbx() {
        let mut ctx = AzContext::new(AzDbType::Mdbx, 10000);
        
        let result = ctx.authorize(
            "d:test:resource",
            "d:test:user",
            1,
            false
        );
        
        assert!(result.is_ok());
    }
}
```

## Integration Tests

### Multi-threaded Authorization

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    fn test_concurrent_authorization() {
        let ctx = Arc::new(Mutex::new(
            AzContext::new(AzDbType::Lmdb, 10000)
        ));
        
        let mut handles = vec![];
        
        for i in 0..10 {
            let ctx_clone = Arc::clone(&ctx);
            let handle = thread::spawn(move || {
                let mut ctx = ctx_clone.lock().unwrap();
                
                ctx.authorize(
                    &format!("d:test:resource{}", i),
                    "d:test:user",
                    1,
                    false
                )
            });
            handles.push(handle);
        }
        
        for handle in handles {
            let result = handle.join().unwrap();
            assert!(result.is_ok());
        }
    }
}
```

### Cache Performance

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_cache_performance() {
        // Without cache
        let mut ctx_no_cache = LmdbAzContext::new_with_config(
            10000, None, None, Some(false)
        );
        
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = ctx_no_cache.authorize(
                "d:test:cached_resource",
                "d:test:user",
                1,
                false
            );
        }
        let time_no_cache = start.elapsed();
        
        // With cache
        let mut ctx_with_cache = LmdbAzContext::new_with_config(
            10000, None, None, Some(true)
        );
        
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = ctx_with_cache.authorize(
                "d:test:cached_resource",
                "d:test:user",
                1,
                false
            );
        }
        let time_with_cache = start.elapsed();
        
        println!("Without cache: {:?}", time_no_cache);
        println!("With cache: {:?}", time_with_cache);
        
        // Cache should be faster (or equal if cache not effective)
        assert!(time_with_cache <= time_no_cache);
    }
}
```

### Counter and Reconnection

```rust
#[cfg(test)]
mod counter_tests {
    use super::*;

    #[test]
    fn test_counter_increment() {
        let mut ctx = LmdbAzContext::new(10);
        
        // First call: counter = 1
        let _ = ctx.authorize("d:test:r1", "d:test:u1", 1, false);
        assert_eq!(ctx.authorize_counter, 1);
        
        // Second call: counter = 2
        let _ = ctx.authorize("d:test:r2", "d:test:u2", 1, false);
        assert_eq!(ctx.authorize_counter, 2);
    }

    #[test]
    fn test_counter_reset() {
        let mut ctx = LmdbAzContext::new(3);
        
        // Make 3 calls
        for i in 0..3 {
            let _ = ctx.authorize(
                &format!("d:test:r{}", i),
                "d:test:u",
                1,
                false
            );
        }
        
        // Counter should have reset
        assert_eq!(ctx.authorize_counter, 0);
    }
}
```

## Test Cleanup

### Resetting Global State

```rust
#[cfg(test)]
mod cleanup_tests {
    use super::*;

    #[test]
    fn test_with_cleanup() {
        {
            let mut ctx = LmdbAzContext::new(10000);
            let _ = ctx.authorize("d:test:r", "d:test:u", 1, false);
        } // Context dropped
        
        // Force sync before reset
        sync_lmdb_env();
        
        // Reset global environments
        reset_lmdb_global_envs();
        
        // Create new context (will reinitialize)
        let mut ctx = LmdbAzContext::new(10000);
        let result = ctx.authorize("d:test:r", "d:test:u", 1, false);
        assert!(result.is_ok());
    }
}
```

### Test Isolation

```rust
#[cfg(test)]
mod isolation_tests {
    use super::*;
    use std::sync::Mutex;
    
    // Global test lock to prevent concurrent test execution
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_isolated_lmdb() {
        let _guard = TEST_LOCK.lock().unwrap();
        
        // Test logic here
        let mut ctx = LmdbAzContext::new(10000);
        // ...
        
        // Cleanup
        sync_lmdb_env();
        reset_lmdb_global_envs();
    }
    
    #[test]
    fn test_isolated_mdbx() {
        let _guard = TEST_LOCK.lock().unwrap();
        
        // Test logic here
        let mut ctx = MdbxAzContext::new(10000);
        // ...
        
        // Cleanup
        sync_mdbx_env();
        reset_mdbx_global_envs();
    }
}
```

## Benchmark Tests

### Authorization Throughput

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[test]
    fn benchmark_lmdb_throughput() {
        let mut ctx = LmdbAzContext::new(100000);
        
        let iterations = 10000;
        let start = Instant::now();
        
        for i in 0..iterations {
            let _ = ctx.authorize(
                &format!("d:test:resource{}", i % 100),
                "d:test:user",
                1,
                false
            );
        }
        
        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
        
        println!("LMDB Throughput: {:.2} ops/sec", ops_per_sec);
        println!("Average latency: {:.2} μs", 
                 elapsed.as_micros() as f64 / iterations as f64);
    }

    #[test]
    fn benchmark_mdbx_throughput() {
        let mut ctx = MdbxAzContext::new(100000);
        
        let iterations = 10000;
        let start = Instant::now();
        
        for i in 0..iterations {
            let _ = ctx.authorize(
                &format!("d:test:resource{}", i % 100),
                "d:test:user",
                1,
                false
            );
        }
        
        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
        
        println!("MDBX Throughput: {:.2} ops/sec", ops_per_sec);
        println!("Average latency: {:.2} μs", 
                 elapsed.as_micros() as f64 / iterations as f64);
    }
}
```

### Backend Comparison

```rust
#[cfg(test)]
mod comparison_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn compare_backends() {
        let iterations = 1000;
        let resources: Vec<String> = (0..100)
            .map(|i| format!("d:test:resource{}", i))
            .collect();
        
        // LMDB
        let mut lmdb_ctx = LmdbAzContext::new(100000);
        let lmdb_start = Instant::now();
        for i in 0..iterations {
            let _ = lmdb_ctx.authorize(
                &resources[i % resources.len()],
                "d:test:user",
                1,
                false
            );
        }
        let lmdb_time = lmdb_start.elapsed();
        
        // MDBX
        let mut mdbx_ctx = MdbxAzContext::new(100000);
        let mdbx_start = Instant::now();
        for i in 0..iterations {
            let _ = mdbx_ctx.authorize(
                &resources[i % resources.len()],
                "d:test:user",
                1,
                false
            );
        }
        let mdbx_time = mdbx_start.elapsed();
        
        println!("LMDB: {:?} ({:.2} ops/sec)", 
                 lmdb_time, 
                 iterations as f64 / lmdb_time.as_secs_f64());
        println!("MDBX: {:?} ({:.2} ops/sec)", 
                 mdbx_time, 
                 iterations as f64 / mdbx_time.as_secs_f64());
    }
}
```

## Debugging Tests

### Enable Logging

```rust
#[cfg(test)]
mod debug_tests {
    use super::*;
    use env_logger;

    #[test]
    fn test_with_logging() {
        // Initialize logger
        let _ = env_logger::builder()
            .is_test(true)
            .try_init();
        
        let mut ctx = LmdbAzContext::new(10000);
        
        let result = ctx.authorize(
            "d:test:resource",
            "d:test:user",
            1,
            false
        );
        
        assert!(result.is_ok());
    }
}
```

### Trace Authorization

```rust
#[cfg(test)]
mod trace_tests {
    use super::*;
    use v_authorization::common::Trace;

    #[test]
    fn test_authorization_trace() {
        let mut ctx = LmdbAzContext::new(10000);
        
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
            "d:test:resource",
            "d:test:user",
            1,
            false,
            &mut trace
        );
        
        println!("Result: {:?}", result);
        println!("ACL: {}", acl);
        println!("Groups: {}", group);
        println!("Info: {}", info);
        
        assert!(result.is_ok());
    }
}
```

## Running Tests

### Run All Tests

```bash
cargo test
```

### Run Specific Test Module

```bash
cargo test tests::test_lmdb_basic_authorization
```

### Run With Output

```bash
cargo test -- --nocapture
```

### Run Benchmarks

```bash
cargo test --release benchmark
```

### Run With Logging

```bash
RUST_LOG=debug cargo test -- --nocapture
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Setup test databases
        run: |
          mkdir -p ./data/acl-indexes
          mkdir -p ./data/acl-cache-indexes
          mkdir -p ./data/acl-mdbx-indexes
          mkdir -p ./data/acl-cache-mdbx-indexes
      
      - name: Run tests
        run: cargo test --verbose
      
      - name: Run benchmarks
        run: cargo test --release benchmark
```

## Troubleshooting Tests

### Database Not Found

```rust
// Ensure databases exist before tests
#[cfg(test)]
mod setup {
    use std::path::Path;
    
    #[ctor::ctor]
    fn setup() {
        std::fs::create_dir_all("./data/acl-indexes").unwrap();
        std::fs::create_dir_all("./data/acl-cache-indexes").unwrap();
        std::fs::create_dir_all("./data/acl-mdbx-indexes").unwrap();
        std::fs::create_dir_all("./data/acl-cache-mdbx-indexes").unwrap();
    }
}
```

### Test Interference

If tests interfere with each other:

1. Use test isolation with locks
2. Reset global state between tests
3. Run tests serially: `cargo test -- --test-threads=1`

### Memory Leaks

If tests show memory leaks:

1. Ensure contexts are properly dropped
2. Call sync functions before reset
3. Check for circular Arc references

