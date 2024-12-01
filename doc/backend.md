# Backend API Documentation

The Backend struct provides access to core storage, full-text search, and authentication functionalities in Veda. It serves as a central access point for data operations.

## Table of Contents

1. [Initialization](#initialization)
2. [Core Functions](#core-functions)
3. [Core Component Interfaces](#core-component-interfaces)
4. [Error Handling](#error-handling)
5. [Configuration](#configuration)
6. [Best Practices](#best-practices)

## Initialization

### `Backend::create(storage_mode: StorageMode, use_remote_storage: bool) -> Self`

Creates a new Backend instance with specified storage configuration.

Parameters:
- `storage_mode`: Determines read/write permissions (ReadOnly/ReadWrite)
- `use_remote_storage`: If true, uses remote storage connection instead of local

Example:
```rust
let backend = Backend::create(StorageMode::ReadOnly, false);
```

### `Backend::default() -> Self`

Creates a new Backend instance with default settings (ReadOnly mode and local storage).

```rust
let backend = Backend::default();
```

## Core Functions

### `get_sys_ticket_id(&mut self) -> Result<String, i32>`

Retrieves the system ticket ID from the database.

```rust
match backend.get_sys_ticket_id() {
    Ok(ticket_id) => println!("System ticket: {}", ticket_id),
    Err(code) => println!("Error getting ticket: {}", code)
}
```

### `get_literal_of_link(&mut self, indv: &mut Individual, link: &str, field: &str, to: &mut Individual) -> Option<String>`

Gets the first literal value from a linked individual.

Parameters:
- `indv`: Source individual containing the link
- `link`: Name of the link predicate
- `field`: Name of the field to retrieve from linked individual
- `to`: Buffer individual for storing the linked entity

```rust
let mut target = Individual::default();
if let Some(value) = backend.get_literal_of_link(&mut source, "v-s:creator", "v-s:name", &mut target) {
    println!("Creator name: {}", value);
}
```

### `get_literals_of_link(&mut self, indv: &mut Individual, link: &str, field: &str) -> Vec<String>`

Gets all literal values from linked individuals.

Parameters:
- `indv`: Source individual containing the links
- `link`: Name of the link predicate
- `field`: Name of the field to retrieve from linked individuals

```rust
let values = backend.get_literals_of_link(&mut source, "v-s:hasComment", "v-s:text");
for value in values {
    println!("Comment text: {}", value);
}
```

### `get_datetime_of_link(&mut self, indv: &mut Individual, link: &str, field: &str, to: &mut Individual) -> Option<i64>`

Gets a datetime value from a linked individual.

Parameters:
- `indv`: Source individual containing the link
- `link`: Name of the link predicate
- `field`: Name of the datetime field to retrieve
- `to`: Buffer individual for storing the linked entity

```rust
let mut target = Individual::default();
if let Some(timestamp) = backend.get_datetime_of_link(&mut source, "v-s:created", "v-s:date", &mut target) {
    println!("Creation timestamp: {}", timestamp);
}
```

### `get_individual_h(&mut self, uri: &str) -> Option<Box<Individual>>`

Retrieves an individual by URI and returns it as a heap-allocated box.

```rust
if let Some(individual) = backend.get_individual_h("document:123") {
    println!("Found document: {}", individual.get_id());
}
```

### `get_individual_s(&mut self, uri: &str) -> Option<Individual>`

Retrieves an individual by URI and returns it as a stack-allocated value.

```rust
if let Some(individual) = backend.get_individual_s("document:123") {
    println!("Found document: {}", individual.get_id());
}
```

### `get_individual<'a>(&mut self, uri: &str, iraw: &'a mut Individual) -> Option<&'a mut Individual>`

Retrieves an individual by URI into a provided buffer.

Parameters:
- `uri`: URI of the individual to retrieve
- `iraw`: Buffer to store the retrieved individual

```rust
let mut individual = Individual::default();
if let Some(ind) = backend.get_individual("document:123", &mut individual) {
    println!("Found document: {}", ind.get_id());
}
```

### `get_ticket_from_db(&mut self, id: &str) -> Ticket`

Retrieves a ticket from the database by ID.

```rust
let ticket = backend.get_ticket_from_db("ticket:123");
if ticket.result == ResultCode::Ok {
    println!("Valid ticket for user: {}", ticket.user_uri);
}
```

## Core Component Interfaces

### Storage (VStorage)

The VStorage component provides direct access to the storage layer for reading and writing individuals.

#### Key Methods:

```rust
// Get individual by ID
fn get_individual(&mut self, id: &str, individual: &mut Individual) -> ResultCode;

// Get individual from specific database
fn get_individual_from_db(&mut self, storage_id: StorageId, id: &str, individual: &mut Individual) -> ResultCode;

// Get raw binary value
fn get_raw_value(&mut self, storage_id: StorageId, key: &str) -> Vec<u8>;
```

Usage example:
```rust
let mut individual = Individual::default();
if backend.storage.get_individual("doc:123", &mut individual) == ResultCode::Ok {
    println!("Found document: {}", individual.get_id());
}
```

### Full-Text Search (FTClient)

The FTClient provides interfaces for text-based searching and querying.

#### Key Methods:

```rust
// Query individuals using full text search
fn query(
    &mut self, 
    user: &str, 
    query: &str, 
    sort: &str, 
    databases: &str, 
    from: i32, 
    limit: i32, 
    authorized: bool,
) -> QueryResult;

// Count results for a query
fn query_count(&mut self, user: &str, query: &str, databases: &str, authorized: bool) -> i64;
```

Usage example:
```rust
let result = backend.fts.query(
    "user:admin",        // user
    "v-s:creator == 'user:john'",  // query
    "v-s:created",       // sort
    "individuals",       // databases
    0,                   // from
    100,                // limit
    true,               // authorized
);

for hit in result.result {
    println!("Found: {}, Score: {}", hit.id, hit.score);
}
```

### Main Storage API (MStorageClient)

The MStorageClient provides high-level operations for managing individuals in the main storage.

#### Key Methods:

```rust
// Get individual by ID
fn get_individual(&mut self, uri: &str, user_uri: &str) -> IOResult;

// Update individual
fn update_use_param(&mut self, ticket: &str, cmd: IndvOp, indv: &Individual) -> ResultCode;

// Add to individual
fn add_to_individual(&mut self, ticket: &str, indv: &Individual) -> ResultCode;

// Remove from individual
fn remove_from_individual(&mut self, ticket: &str, indv: &Individual) -> ResultCode;
```

Usage example:
```rust
// Get individual using main storage API
let result = backend.mstorage_api.get_individual("doc:123", "user:admin");
if result.result == ResultCode::Ok {
    println!("Found document: {}", result.id);
}

// Update individual
let mut indv = Individual::default();
indv.set_id("doc:123");
indv.add_uri("rdf:type", "v-s:Document");
let res = backend.mstorage_api.update_use_param(
    "ticket:123",
    IndvOp::Put,
    &indv
);
```

### Authentication API (AuthClient)

The AuthClient handles authentication and authorization operations.

#### Key Methods:

```rust
// Get ticket
fn get_ticket_trust(&mut self, login: &str, password: &str, user_uri: &str) -> Result<Ticket, i32>;

// Check rights
fn access_check(&mut self, user_uri: &str, uri: &str, user_groups: Vec<String>) -> i32;

// Get rights origin
fn get_rights_origin(
    &mut self, 
    user_uri: &str, 
    uri: &str, 
    user_groups: Vec<String>
) -> Vec<String>;
```

Usage example:
```rust
// Get ticket for user
match backend.auth_api.get_ticket_trust("user1", "pass123", "user:john") {
    Ok(ticket) => {
        println!("Got ticket: {}", ticket.id);
    },
    Err(e) => {
        println!("Failed to get ticket: {}", e);
    }
}

// Check access rights
let groups = vec!["group:admin".to_string()];
let access_level = backend.auth_api.access_check(
    "user:john",
    "doc:123",
    groups
);
```

## Error Handling

Each component has its own error handling approach:

```rust
// VStorage typically returns ResultCode
match backend.storage.get_individual("doc:123", &mut indv) {
    ResultCode::Ok => { /* handle success */ },
    ResultCode::NotFound => { /* handle not found */ },
    _ => { /* handle other errors */ }
}

// FTClient returns QueryResult with error information
let search_result = backend.fts.query(/* params */);
if search_result.error.is_empty() {
    // Process results
} else {
    // Handle error
}

// MStorageClient operations return ResultCode
match backend.mstorage_api.update_use_param(ticket, cmd, &indv) {
    ResultCode::Ok => { /* handle success */ },
    ResultCode::AuthenticationFailed => { /* handle auth failure */ },
    _ => { /* handle other errors */ }
}

// AuthClient operations typically return Result
match backend.auth_api.get_ticket_trust(login, pass, uri) {
    Ok(ticket) => { /* use ticket */ },
    Err(error_code) => { /* handle error */ }
}
```

## Configuration

The Backend reads configuration from the following sources:
- Command line arguments
- veda.properties file
- Environment variables

Key configuration parameters:
- `ft_query_service_url`: URL for the full-text search service
- `ro_storage_url`: URL for remote storage (when use_remote_storage is true)
- `main_module_url`: URL for the main storage module
- `auth_url`: URL for the authentication service

### Configuration Best Practices

1. **Connection Settings**:
   - Use appropriate timeouts for network operations
   - Configure connection pools appropriately
   - Use secure connections where available

2. **Performance Tuning**:
   - Configure batch sizes appropriately
   - Set reasonable limits for search operations
   - Monitor and adjust cache sizes as needed

3. **Security**:
   - Use appropriate access controls
   - Validate all inputs
   - Log security-relevant operations

## Best Practices

### General Best Practices

1. Always use the most appropriate get_individual variant for your use case:
   - Use `get_individual_h` for long-lived individuals
   - Use `get_individual_s` for temporary access
   - Use `get_individual` when you have a buffer to reuse

2. Check ticket validity before processing protected operations

3. Use proper error handling for all operations that return Option or Result

4. Consider using the storage mode that best fits your needs:
   - ReadOnly for query operations
   - ReadWrite for modifications

5. Monitor and log errors appropriately using the standard logging facilities

### Component-Specific Best Practices

1. **Storage Access**:
   - Use direct storage access (VStorage) for read-only operations
   - Use MStorageClient for operations that need to maintain consistency
   - Consider caching frequently accessed individuals

2. **Full-Text Search**:
   - Build efficient queries to minimize search time
   - Use pagination (from/limit) for large result sets
   - Consider whether authorization is needed for the search

3. **Authentication**:
   - Cache tickets when appropriate
   - Validate tickets before performing protected operations
   - Use proper error handling for authentication failures

4. **General**:
   - Log important operations and errors
   - Handle all error cases appropriately
   - Consider using transactions where available
   - Monitor performance and optimize queries as needed

