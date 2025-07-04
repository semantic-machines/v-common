// examples/storage_types.rs
//! Demonstration of different storage types in v-storage
//! 
//! This example shows:
//! - Dynamic storage (VStorage) - trait objects
//! - Generic storage (VStorageGeneric) - compile-time typing
//! - Enum storage (VStorageEnum) - static dispatch
//! - Performance and usage comparison

use v_storage::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== v-storage Storage Types Comparison ===\n");

    // === 1. Dynamic storage (VStorage) ===
    println!("🎭 1. Dynamic storage (VStorage)");
    println!("   • Uses trait objects (Box<dyn Storage>)");
    println!("   • Runtime flexibility, but has vtable lookup overhead");
    
    let storage_box = VStorage::builder().memory().build()?;
    let mut dynamic_storage = VStorage::new(storage_box);
    
    // Demonstrate operations
    let _ = dynamic_storage.put_value(StorageId::Individuals, "dynamic:key", "dynamic:value");
    
    if let StorageResult::Ok(value) = dynamic_storage.get_value(StorageId::Individuals, "dynamic:key") {
        println!("   • Stored and read: {}", value);
    }
    
    println!("   • Storage is empty: {}", dynamic_storage.is_empty());

    // === 2. Generic storage (VStorageGeneric) ===
    println!("\n🔧 2. Generic storage (VStorageGeneric)");
    println!("   • Compile-time typing");
    println!("   • No vtable overhead, but less flexible");
    
    let mut generic_storage = VMemoryStorage::new(memory_storage::MemoryStorage::new());
    
    let _ = generic_storage.put_value(StorageId::Individuals, "generic:key", "generic:value");
    
    if let StorageResult::Ok(value) = generic_storage.get_value(StorageId::Individuals, "generic:key") {
        println!("   • Stored and read: {}", value);
    }
    
    println!("   • Storage is empty: {}", generic_storage.is_empty());
    
    // Can extract inner storage
    if let Some(inner_storage) = generic_storage.storage() {
        println!("   • Access to inner storage: available");
    }

    // === 3. Enum storage (VStorageEnum) ===
    println!("\n⚡ 3. Enum storage (VStorageEnum)");
    println!("   • Static dispatch through enum");
    println!("   • Static dispatch");
    
    let mut enum_storage = VStorageEnum::memory();
    
    let _ = enum_storage.put_value(StorageId::Individuals, "enum:key", "enum:value");
    
    if let StorageResult::Ok(value) = enum_storage.get_value(StorageId::Individuals, "enum:key") {
        println!("   • Stored and read: {}", value);
    }
    
    println!("   • Storage is empty: {}", enum_storage.is_empty());

    // === 4. API consistency ===
    println!("\n🔄 4. API consistency between types");
    println!("   • All types have the same Storage interface");
    
    let test_data = vec![
        ("test1", "value1"),
        ("test2", "value2"),
        ("test3", "value3"),
    ];

    // Store data in all storage types
    for (key, value) in &test_data {
        let _ = dynamic_storage.put_value(StorageId::Tickets, key, value);
        let _ = generic_storage.put_value(StorageId::Tickets, key, value);
        let _ = enum_storage.put_value(StorageId::Tickets, key, value);
    }

    // Check that all return the same results
    println!("   • Consistency check:");
    for (key, expected_value) in &test_data {
        let dynamic_result = dynamic_storage.get_value(StorageId::Tickets, key);
        let generic_result = generic_storage.get_value(StorageId::Tickets, key);
        let enum_result = enum_storage.get_value(StorageId::Tickets, key);

        match (dynamic_result, generic_result, enum_result) {
            (StorageResult::Ok(v1), StorageResult::Ok(v2), StorageResult::Ok(v3)) => {
                let consistent = v1 == *expected_value && v2 == *expected_value && v3 == *expected_value;
                println!("     {} -> {}", key, if consistent { "✅ consistent" } else { "❌ inconsistent" });
            }
            _ => println!("     {} -> ❌ read error", key),
        }
    }

    // === 5. Different constructor demonstrations ===
    println!("\n🏗️  5. Different ways to create storages");
    
    // VStorageEnum - direct creation
    let _enum_direct = VStorageEnum::memory();
    println!("   • VStorageEnum::memory() - direct creation");
    
    // VStorageGeneric - through constructor
    let _generic_direct = VMemoryStorage::new(memory_storage::MemoryStorage::new());
    println!("   • VMemoryStorage::new() - through constructor");
    
    // VStorage - through builder
    let _dynamic_builder = VStorage::builder().memory().build()?;
    println!("   • VStorage::builder() - through builder pattern");

    // === 6. Features of each type ===
    println!("\n📋 6. Usage characteristics");
    
    println!("   VStorage (Dynamic):");
    println!("     ✅ Maximum flexibility");
    println!("     ✅ Can change storage type at runtime");
    println!("     ❌ Vtable lookup overhead");
    println!("     ❌ No access to inner type");
    
    println!("   VStorageGeneric<T> (Generic):");
    println!("     ✅ Compile-time optimization");
    println!("     ✅ Access to inner storage");
    println!("     ✅ Type safety");
    println!("     ❌ Less flexible at runtime");
    
    println!("   VStorageEnum (Enum):");
    println!("     ✅ Static dispatch");
    println!("     ✅ No heap allocation for storage");
    println!("     ❌ Fixed set of types");

    // === 7. Usage recommendations ===
    println!("\n💡 7. Storage type selection recommendations");
    
    println!("   Use VStorage when:");
    println!("     • Maximum flexibility is needed");
    println!("     • Storage type is determined at runtime");
    println!("     • Flexibility is more important than dispatch overhead");
    
    println!("   Use VStorageGeneric when:");
    println!("     • Access to inner storage is needed");
    println!("     • Type safety is important");
    println!("     • Storage type is known at compile time");
    
    println!("   Use VStorageEnum when:");
    println!("     • Static dispatch is preferred");
    println!("     • Doing many operations (batch processing)");
    println!("     • Fixed set of types is suitable");

    println!("\n✨ Demonstration completed!");
    Ok(())
} 