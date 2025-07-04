// examples/basic_usage.rs
//! Basic usage example for v-storage library
//! 
//! This example demonstrates:
//! - Creating memory storage
//! - Basic operations: put, get, remove, count
//! - Working with different data types (strings, binary data)
//! - Using different StorageId types

use v_storage::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Basic v-storage Usage Example ===\n");

    // Create memory storage using Builder pattern
    let storage = VStorage::builder()
        .memory()
        .build()?;
    let mut storage = VStorage::new(storage);

    println!("✅ Storage created successfully");

    // === Basic data operations ===
    
    // 1. Storing data
    println!("\n📝 Storing data:");
    
    let _ = storage.put_value(StorageId::Individuals, "user:1", r#"{
        "@": "user:1", 
        "rdf:type": [{"type": "Uri", "data": "foaf:Person"}],
        "rdfs:label": [{"type": "String", "data": "Ivan Petrov"}],
        "foaf:name": [{"type": "String", "data": "Ivan"}],
        "foaf:familyName": [{"type": "String", "data": "Petrov"}], 
        "foaf:age": [{"type": "Integer", "data": 30}],
        "foaf:mbox": [{"type": "String", "data": "ivan.petrov@example.com"}],
        "veda:created": [{"type": "Datetime", "data": "2024-01-15T10:30:00Z"}]
    }"#);
    let _ = storage.put_value(StorageId::Tickets, "ticket:123", r#"{
        "@": "ticket:123",
        "rdf:type": [{"type": "Uri", "data": "veda:Ticket"}],
        "rdfs:label": [{"type": "String", "data": "Support Request"}],
        "dcterms:title": [{"type": "String", "data": "Email server issue"}],
        "veda:status": [{"type": "String", "data": "pending"}],
        "veda:priority": [{"type": "String", "data": "medium"}],
        "dcterms:created": [{"type": "Datetime", "data": "2024-01-15T09:15:00Z"}]
    }"#);
    let _ = storage.put_value(StorageId::Az, "permission:read", r#"{
        "@": "permission:read",
        "rdf:type": [{"type": "Uri", "data": "veda:Permission"}],
        "rdfs:label": [{"type": "String", "data": "Read Access"}],
        "veda:permissionSubject": [{"type": "Uri", "data": "user:1"}],
        "veda:permissionObject": [{"type": "Uri", "data": "resource:documents"}],
        "veda:canRead": [{"type": "Boolean", "data": true}],
        "dcterms:created": [{"type": "Datetime", "data": "2024-01-15T08:00:00Z"}]
    }"#);

    println!("   • Stored user: user:1");
    println!("   • Stored ticket: ticket:123");
    println!("   • Stored permission: permission:read");

    // 2. Reading data
    println!("\n📖 Reading data:");
    
    if let StorageResult::Ok(user_data) = storage.get_value(StorageId::Individuals, "user:1") {
        println!("   • User Individual ({}): Ivan Petrov", user_data.len());
    }
    
    if let StorageResult::Ok(ticket_data) = storage.get_value(StorageId::Tickets, "ticket:123") {
        println!("   • Ticket Individual ({}): Support Request", ticket_data.len());
    }
    
    if let StorageResult::Ok(permission_data) = storage.get_value(StorageId::Az, "permission:read") {
        println!("   • Permission Individual ({}): Read Access", permission_data.len());
    }

    // 3. Working with binary data
    println!("\n🔢 Working with binary data:");
    
    let binary_data = vec![0xFF, 0xFE, 0xFD, 0x00, 0x01, 0x02];
    let _ = storage.put_raw_value(StorageId::Individuals, "binary:data", binary_data.clone());
    
    if let StorageResult::Ok(retrieved_binary) = storage.get_raw_value(StorageId::Individuals, "binary:data") {
        println!("   • Stored {} bytes of binary data", retrieved_binary.len());
        println!("   • Data matches: {}", retrieved_binary == binary_data);
    }

    // 4. Counting records
    println!("\n📊 Statistics:");
    
    if let StorageResult::Ok(individuals_count) = storage.count(StorageId::Individuals) {
        println!("   • Individuals count: {}", individuals_count);
    }
    
    if let StorageResult::Ok(tickets_count) = storage.count(StorageId::Tickets) {
        println!("   • Tickets count: {}", tickets_count);
    }
    
    if let StorageResult::Ok(az_count) = storage.count(StorageId::Az) {
        println!("   • Permissions count: {}", az_count);
    }

    // 5. Removing data
    println!("\n🗑️  Removing data:");
    
    let _ = storage.remove_value(StorageId::Tickets, "ticket:123");
    println!("   • Ticket ticket:123 removed");
    
    // Check that data is actually removed
    match storage.get_value(StorageId::Tickets, "ticket:123") {
        StorageResult::NotFound => println!("   • Confirmed: ticket not found"),
        _ => println!("   • Error: ticket still exists"),
    }

    // 6. Error handling
    println!("\n⚠️  Error handling:");
    
    match storage.get_value(StorageId::Individuals, "nonexistent") {
        StorageResult::NotFound => println!("   • Correctly handled missing key case"),
        StorageResult::Ok(_) => println!("   • Unexpected: found data"),
        StorageResult::Error(e) => println!("   • Error: {}", e),
        _ => println!("   • Other result"),
    }

    // === Backward compatibility demonstration ===
    println!("\n🔄 Backward compatibility:");
    
    #[allow(deprecated)]
    {
        // Using old methods
        let success = storage.put_kv(StorageId::Individuals, "old:key", "old:value");
        println!("   • Old put_kv: {}", if success { "success" } else { "error" });
        
        if let Some(value) = storage.get_v(StorageId::Individuals, "old:key") {
            println!("   • Old get_v: {}", value);
        }
    }

    println!("\n✨ Example completed successfully!");
    Ok(())
} 