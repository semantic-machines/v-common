# Complete Individual Usage Guide

`Individual` is a comprehensive data structure in Rust for managing semantic data with support for multiple serialization formats, predicates, and resources. This guide covers all public functionality.

## Table of Contents
- [Core Concepts](#core-concepts)
- [Creation and Basic Operations](#creation-and-basic-operations)
- [Data Management](#data-management)
- [Serialization Formats](#serialization-formats)
- [Value Operations](#value-operations)
- [Query and Retrieval](#query-and-retrieval)
- [Resource Operations](#resource-operations)

## Core Concepts

- **Individual**: A container holding URI identifier and predicates with their resources
- **IndividualObj**: The underlying object storing the actual data
- **RawObj**: Container for raw binary data with parsing state
- **Predicates**: Named properties that can hold multiple values
- **Resources**: Typed values associated with predicates
- **Supported Formats**: CBOR, JSON, MessagePack, and Turtle

## Creation and Basic Operations

### Creating Individuals
```rust
// Empty individual
let mut indv = Individual::default();

// From raw data
let raw = RawObj::new(vec![/* binary data */]);
let mut indv = Individual::new_raw(raw);

// From existing IndividualObj
let mut indv = Individual::new_from_obj(&existing_obj);

// Create empty RawObj
let raw = RawObj::new_empty();
```

### Identity Management
```rust
// Set identifier
indv.set_id("example:123");

// Get identifier
let id = indv.get_id();
```

## Data Management

### Raw Data Operations
```rust
// Set raw data
indv.set_raw(&[/* binary data */]);

// Get raw data length
let len = indv.get_raw_len();

// Parse all data
indv.parse_all();

// Check if individual is empty
indv.is_empty();
```

### Getting Object Reference
```rust
// Get reference to underlying IndividualObj
let obj = indv.get_obj();
```

## Serialization Formats

### JSON Operations
```rust
use crate::onto::json2individual;
use serde_json::Value as JSONValue;

// Parse from JSON
json2individual::parse_json_to_individual(&json_value, &mut indv);

// Convert to JSON string
let json_str = indv.obj.as_json_str();

// Convert to JSON value
let json_value = indv.obj.as_json();
```

### MessagePack Operations
```rust
use crate::onto::individual2msgpack;
use crate::onto::msgpack2individual;

// Convert to MessagePack
let mut output = Vec::new();
individual2msgpack::to_msgpack(&indv, &mut output);

// Parse from MessagePack
msgpack2individual::parse_msgpack(&mut raw_obj);
```

### Turtle Format Operations
```rust
use crate::onto::individual2turtle;

// Convert to Turtle format
let prefixes = HashMap::new(); // Define your prefixes
let individuals = vec![&indv];
let turtle_data = individual2turtle::to_turtle_with_counter_refs(&individuals, &prefixes);
```

### CBOR Operations
```rust
use crate::onto::cbor2individual;

// Parse from CBOR
cbor2individual::parse_cbor(&mut raw_obj);
cbor2individual::parse_cbor_to_predicate("predicate", &mut indv);
```

## Value Operations

### String Operations
```rust
// Add/Set strings with language support
indv.add_string("predicate", "value", Lang::new_from_str("EN"));
indv.set_string("predicate", "value", Lang::new_from_str("RU"));

// URI operations
indv.add_uri("predicate", "http://example.org/resource");
indv.set_uri("predicate", "http://example.org/resource");
indv.set_uris("predicate", vec!["uri1".to_string(), "uri2".to_string()]);
```

### Numeric Operations
```rust
// Integer operations
indv.add_integer("predicate", 42);
indv.set_integer("predicate", 42);

// Decimal operations with different input formats
indv.add_decimal_d("predicate", 1234, -2); // 12.34
indv.set_decimal_d("predicate", 1234, -2);
indv.add_decimal_from_str("predicate", "12.34");
indv.add_decimal_from_i64("predicate", 42);
indv.add_decimal_from_f64("predicate", 12.34);
```

### DateTime Operations
```rust
// Add/Set datetime values
indv.add_datetime("predicate", 1634567890);
indv.set_datetime("predicate", 1634567890);
indv.add_datetime_from_str("predicate", "2023-10-18T15:30:00");
```

### Boolean and Binary Operations
```rust
// Boolean operations
indv.add_bool("predicate", true);
indv.set_bool("predicate", false);

// Binary data operations
indv.add_binary("predicate", vec![1, 2, 3]);
indv.set_binary("predicate", vec![1, 2, 3]);
```

## Query and Retrieval

### Value Retrieval
```rust
// Get first value of different types
indv.get_first_literal("predicate"); // Option<String>
indv.get_first_literal_or_err("predicate"); // Result<String, Error>
indv.get_first_literal_with_lang("predicate", &[Lang::new_from_str("EN")]);
indv.get_first_integer("predicate"); // Option<i64>
indv.get_first_bool("predicate"); // Option<bool>
indv.get_first_datetime("predicate"); // Option<i64>
indv.get_first_number("predicate"); // Option<(i64, i64)>
indv.get_first_float("predicate"); // Option<f64>
indv.get_first_binobj("predicate"); // Option<Vec<u8>>
```

### Multiple Values Retrieval
```rust
// Get all resources or literals
indv.get_resources("predicate"); // Option<Vec<Resource>>
indv.get_literals("predicate"); // Option<Vec<String>>
indv.get_literals_nm("predicate"); // Option<Vec<String>> without parsing

// Get all predicates
indv.get_predicates(); // Vec<String>
indv.get_predicates_nm(); // Vec<String> without parsing
indv.get_predicates_of_type(DataType::String); // Vec<String>
```

### Existence Checks
```rust
// Check predicate existence
indv.is_exists("predicate");

// Check specific values
indv.any_exists("predicate", &["value1", "value2"]);
indv.any_exists_v("predicate", &vec_of_strings);
indv.is_exists_bool("predicate", true);
```

## Resource Operations

### Predicate Management
```rust
// Remove predicate
indv.remove("predicate");

// Clear predicate values
indv.clear("predicate");
```

### Resource Transfer
```rust
// Apply predicates from another individual
indv.apply_predicate_as_set("predicate", &mut other_indv);
indv.apply_predicate_as_add_unique("predicate", &mut other_indv);
indv.apply_predicate_as_remove("predicate", &mut other_indv);
```

### Comparison
```rust
// Compare individuals with optional predicate ignoring
indv.compare(&other_indv, vec!["ignore_predicate"]);
```

## Best Practices

1. Use appropriate serialization format based on your use case:
   - JSON for human-readable data and web APIs
   - MessagePack for efficient binary serialization
   - CBOR for binary data with schema support
   - Turtle for RDF data representation

2. Handle language tags properly for internationalization:
   - Use `Lang::new_from_str()` for explicit language tags
   - Use `Lang::none()` for language-neutral strings

3. Choose between add_* and set_* methods:
   - `add_*` appends new values
   - `set_*` replaces existing values

4. Use appropriate data retrieval methods:
   - `get_first_*` for single values
   - `get_*` for multiple values
   - `*_nm` variants when parsing isn't needed

5. Handle errors and Options appropriately:
   - Use `get_first_literal_or_err` when you need error details
   - Check Option returns for null values

6. Parse efficiently:
   - Use `parse_all()` when you need all data
   - Use targeted parsing for specific predicates

7. Consider memory usage:
   - Clear unnecessary predicates
   - Use appropriate data types
   - Handle binary data carefully
