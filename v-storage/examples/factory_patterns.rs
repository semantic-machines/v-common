// examples/factory_patterns.rs
//! Demonstration of factory patterns for creating storages
//! 
//! This example shows:
//! - Builder Pattern for step-by-step creation
//! - Provider Pattern for ready factory methods
//! - Config Pattern for creation from configuration
//! - Generic builders for compile-time optimization

use v_storage::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== v-storage Factory Patterns Demonstration ===\n");

    // === 1. Builder Pattern ===
    println!("🏗️  1. Builder Pattern - step-by-step creation");
    println!("   • Flexible and readable way to configure storage");
    
    // Create memory storage through builder
    let memory_storage = StorageBuilder::new()
        .memory()
        .build()?;
    println!("   • Created memory storage through builder");
    
    // Create LMDB storage through builder
    let lmdb_storage = StorageBuilder::new()
        .lmdb("/tmp/example_lmdb", StorageMode::ReadWrite, Some(1000))
        .build();
    
    match lmdb_storage {
        Ok(_) => println!("   • Created LMDB storage through builder"),
        Err(e) => println!("   • LMDB unavailable: {}", e),
    }
    
    // Create remote storage through builder
    let remote_storage = StorageBuilder::new()
        .remote("127.0.0.1:8080")
        .build();
    
    match remote_storage {
        Ok(_) => println!("   • Created remote storage through builder"),
        Err(e) => println!("   • Remote unavailable: {}", e),
    }

    // === 2. Provider Pattern ===
    println!("\n🏭 2. Provider Pattern - ready factory methods");
    println!("   • Quick creation of standard configurations");
    
    // Direct creation of storages through Provider
    let _provider_memory = StorageProvider::memory();
    println!("   • StorageProvider::memory() - created");
    
    let _provider_lmdb = StorageProvider::lmdb("/tmp/provider_lmdb", StorageMode::ReadOnly, None);
    println!("   • StorageProvider::lmdb() - created");
    
    let _provider_remote = StorageProvider::remote("127.0.0.1:8080");
    println!("   • StorageProvider::remote() - created");
    
    // Create VStorage through Provider
    let mut vstorage_memory = StorageProvider::vstorage_memory();
    println!("   • StorageProvider::vstorage_memory() - created VStorage");
    
    // Demonstrate functionality with semantic Individual
    let provider_individual = r#"{
        "@": "provider:test",
        "rdf:type": [{"type": "Uri", "data": "foaf:Person"}],
        "rdfs:label": [{"type": "String", "data": "Provider Test User"}],
        "foaf:name": [{"type": "String", "data": "Test"}],
        "veda:createdBy": [{"type": "Uri", "data": "provider:factory"}],
        "dcterms:created": [{"type": "Datetime", "data": "2024-01-15T17:00:00Z"}]
    }"#;
    
    let _ = vstorage_memory.put_value(StorageId::Individuals, "provider:test", provider_individual);
    if let StorageResult::Ok(value) = vstorage_memory.get_value(StorageId::Individuals, "provider:test") {
        println!("     Test: stored Individual ({} bytes) through Provider", value.len());
    }

    // === 3. Config Pattern ===
    println!("\n⚙️  3. Config Pattern - creation from configuration");
    println!("   • Creating storages from configuration structures");
    
    // Create from different configurations
    let configs = vec![
        ("Memory", StorageConfig::Memory),
        ("LMDB", StorageConfig::Lmdb { 
            path: "/tmp/config_lmdb".to_string(), 
            mode: StorageMode::ReadWrite, 
            max_read_counter_reopen: Some(500) 
        }),
        ("Remote", StorageConfig::Remote { 
            address: "127.0.0.1:8080".to_string() 
        }),
    ];
    
    for (name, config) in configs {
        match VStorage::from_config(config) {
            Ok(mut storage) => {
                println!("   • {} configuration: created successfully", name);
                
                // Test the created storage with semantic Individual
                let test_key = format!("config:{}:key", name.to_lowercase());
                let test_value = format!(r#"{{
                    "@": "{}",
                    "rdf:type": [{{"type": "Uri", "data": "veda:ConfigTest"}}],
                    "rdfs:label": [{{"type": "String", "data": "Config Test for {}"}}],
                    "veda:configType": [{{"type": "String", "data": "{}"}}],
                    "dcterms:created": [{{"type": "Datetime", "data": "2024-01-15T17:30:00Z"}}]
                }}"#, test_key, name, name);
                
                let _ = storage.put_value(StorageId::Individuals, &test_key, &test_value);
                if let StorageResult::Ok(retrieved) = storage.get_value(StorageId::Individuals, &test_key) {
                    println!("     Test: {} Individual ({} bytes)", name, retrieved.len());
                }
            },
            Err(e) => println!("   • {} configuration: error - {}", name, e),
        }
    }

    // === 4. Generic Builders ===
    println!("\n🔧 4. Generic Builders - compile-time optimization");
    println!("   • Typed builders without runtime overhead");
    
    // Generic memory builder
    let generic_memory = StorageBuilder::new()
        .memory()
        .build_memory_generic()?;
    println!("   • build_memory_generic() - created VMemoryStorage");
    
    // Generic LMDB builder
    let generic_lmdb = StorageBuilder::new()
        .lmdb("/tmp/generic_lmdb", StorageMode::ReadWrite, None)
        .build_lmdb_generic();
    
    match generic_lmdb {
        Ok(_storage) => println!("   • build_lmdb_generic() - created VLMDBStorage"),
        Err(e) => println!("   • build_lmdb_generic() - error: {}", e),
    }
    
    // Generic remote builder
    let generic_remote = StorageBuilder::new()
        .remote("127.0.0.1:8080")
        .build_remote_generic();
    
    match generic_remote {
        Ok(_storage) => println!("   • build_remote_generic() - created VRemoteStorage"),
        Err(e) => println!("   • build_remote_generic() - error: {}", e),
    }

    // === 5. Provider Generic Methods ===
    println!("\n🎯 5. Provider Generic Methods - static types");
    println!("   • Direct creation of typed storages");
    
    let mut provider_generic_memory = StorageProvider::memory_generic();
    println!("   • StorageProvider::memory_generic() - VMemoryStorage");
    
    let provider_generic_lmdb = StorageProvider::lmdb_generic("/tmp/generic_provider", StorageMode::ReadOnly, None);
    println!("   • StorageProvider::lmdb_generic() - VLMDBStorage");
    
    let provider_generic_remote = StorageProvider::remote_generic("127.0.0.1:8080");
    println!("   • StorageProvider::remote_generic() - VRemoteStorage");
    
    // Demonstrate work with typed storage using semantic Individual
    let generic_individual = r#"{
        "@": "generic:test",
        "rdf:type": [{"type": "Uri", "data": "foaf:Person"}],
        "rdfs:label": [{"type": "String", "data": "Generic Test User"}],
        "foaf:name": [{"type": "String", "data": "Generic"}],
        "veda:storageType": [{"type": "String", "data": "memory_generic"}],
        "dcterms:created": [{"type": "Datetime", "data": "2024-01-15T18:00:00Z"}]
    }"#;
    
    let _ = provider_generic_memory.put_value(StorageId::Individuals, "generic:test", generic_individual);
    if let StorageResult::Ok(value) = provider_generic_memory.get_value(StorageId::Individuals, "generic:test") {
        println!("     Generic test: Individual ({} bytes)", value.len());
    }

    // === 6. Approach comparison ===
    println!("\n📊 6. Storage creation approach comparison");
    
    println!("   Builder Pattern:");
    println!("     ✅ Readable and flexible API");
    println!("     ✅ Validation at build() stage");
    println!("     ✅ Can create both dynamic and generic types");
    println!("     ❌ More code for simple cases");
    
    println!("   Provider Pattern:");
    println!("     ✅ Concise code for standard cases");
    println!("     ✅ Ready optimal configurations");
    println!("     ❌ Less flexible for customization");
    
    println!("   Config Pattern:");
    println!("     ✅ Excellent for loading from files/ENV");
    println!("     ✅ Unified interface for all types");
    println!("     ❌ Only dynamic dispatch");
    
    println!("   Generic Methods:");
    println!("     ✅ Static dispatch");
    println!("     ✅ Compile-time typing");
    println!("     ❌ Less flexible at runtime");

    // === 7. Usage recommendations ===
    println!("\n💡 7. Pattern selection recommendations");
    
    println!("   Use Builder when:");
    println!("     • Parameter customization is needed");
    println!("     • Complex creation logic");
    println!("     • Code readability is needed");
    
    println!("   Use Provider when:");
    println!("     • Need to quickly create standard storage");
    println!("     • Default parameters are suitable");
    println!("     • Minimal boilerplate code");
    
    println!("   Use Config when:");
    println!("     • Configuration is loaded from external sources");
    println!("     • Storage type is determined at runtime");
    println!("     • Uniform handling of different types is needed");
    
    println!("   Use Generic methods when:");
    println!("     • Static dispatch is preferred");
    println!("     • Storage type is known at compile time");
    println!("     • Access to type-specific methods is needed");

    println!("\n✨ Factory patterns demonstration completed!");
    Ok(())
} 