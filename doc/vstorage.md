# Storage System Overview

The storage system provides a unified interface for different storage backends including LMDB, Tarantool, Remote Storage, and In-Memory Storage.

## Core Components

### StorageId
Enum that defines different storage spaces:
- `Individuals` - For storing individual objects
- `Tickets` - For storing tickets
- `Az` - For storing authorization data

### StorageMode
Defines storage access modes:
- `ReadOnly` - Only read operations allowed
- `ReadWrite` - Both read and write operations allowed

### Storage Trait
The main trait that all storage implementations must implement:
```rust
pub trait Storage {
    fn get_individual_from_db(&mut self, storage: StorageId, id: &str, iraw: &mut Individual) -> ResultCode;
    fn get_v(&mut self, storage: StorageId, key: &str) -> Option<String>;
    fn get_raw(&mut self, storage: StorageId, key: &str) -> Vec<u8>;
    fn put_kv(&mut self, storage: StorageId, key: &str, val: &str) -> bool;
    fn put_kv_raw(&mut self, storage: StorageId, key: &str, val: Vec<u8>) -> bool;
    fn remove(&mut self, storage: StorageId, key: &str) -> bool;
    fn count(&mut self, storage: StorageId) -> usize;
}
```

## VStorage Class
Main storage interface that provides unified access to different storage backends.

### Available Storage Backends
1. LMDB Storage - Persistent disk-based storage
2. Tarantool Storage - In-memory database with persistence
3. Remote Storage - Client for remote storage access
4. Memory Storage - Pure in-memory storage for testing and temporary data

### Creating Storage Instances
```rust
// Create LMDB storage
let lmdb = VStorage::new_lmdb("/path/to/db", StorageMode::ReadWrite, None);

// Create Tarantool storage
let tt = VStorage::new_tt("connection_string", "login", "pass");

// Create Remote storage
let remote = VStorage::new_remote("remote_addr");

// Create Memory storage
let memory = VStorage::new_memory();

// Create empty storage
let none = VStorage::none();
```

## Common Usage Patterns

### Individual Operations
```rust
let mut storage = VStorage::new_memory();
let mut individual = Individual::default();

// Read individual
let result = storage.get_individual("id", &mut individual);

// Read from specific storage
let result = storage.get_individual_from_db(StorageId::Individuals, "id", &mut individual);
```

### Key-Value Operations
```rust
// String values
storage.put_kv(StorageId::Individuals, "key", "value");
let value = storage.get_value(StorageId::Individuals, "key");

// Raw bytes
storage.put_kv_raw(StorageId::Individuals, "key", vec![1, 2, 3]);
let raw_value = storage.get_raw_value(StorageId::Individuals, "key");
```

### Testing Support
The memory storage backend is particularly useful for testing:
```rust
#[test]
fn test_storage_operations() {
    let mut storage = VStorage::new_memory();
    // Perform test operations...
}
```

## Important Notes
1. Remote storage supports only read operations
2. Memory storage is not persistent between program restarts
3. LMDB storage requires proper file system permissions
4. Each storage type may return different ResultCodes for the same operation
5. Always check operation results for proper error handling

This documentation should help AI understand the storage system's architecture and typical usage patterns when answering questions or providing code examples.
