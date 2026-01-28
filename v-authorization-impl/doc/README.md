# Documentation Index

## Table of Contents

### Getting Started
- [00. Overview](00_overview.md) - Project introduction and key features
- [01. Getting Started](01_getting_started.md) - Installation and quick start guide

### Configuration and Usage
- [02. Configuration](02_configuration.md) - Detailed configuration options
- [03. API Reference](03_api_reference.md) - Complete API documentation
- [04. Examples](04_examples.md) - Code examples and usage patterns

### Advanced Topics
- [05. Architecture](05_architecture.md) - System design and components
- [06. Implementation Details](06_implementation_details.md) - Internal implementation
- [07. Testing](07_testing.md) - Testing guide and examples
- [08. Troubleshooting](08_troubleshooting.md) - Common issues and solutions

## Quick Links

### For New Users
1. Read [Overview](00_overview.md) to understand what the library does
2. Follow [Getting Started](01_getting_started.md) for installation
3. Try [Examples](04_examples.md) to see code in action
4. Refer to [Configuration](02_configuration.md) for customization

### For Developers
1. Study [Architecture](05_architecture.md) to understand design
2. Review [Implementation Details](06_implementation_details.md) for internals
3. Check [API Reference](03_api_reference.md) for method signatures
4. Read [Testing](07_testing.md) for test patterns

### For Troubleshooting
1. Check [Troubleshooting](08_troubleshooting.md) for common issues
2. Enable logging as described in troubleshooting guide
3. Review [Examples](04_examples.md) for correct usage patterns

## Document Structure

### 00. Overview
- Project description
- Key features
- Architecture overview
- Use cases
- Database paths
- Version information

### 01. Getting Started
- Installation instructions
- Dependencies
- Quick start examples
- Basic configuration
- Next steps

### 02. Configuration
- Configuration options
- max_read_counter
- stat_collector_url
- stat_mode
- use_cache
- Database paths
- Environment setup
- Production and development examples

### 03. API Reference
- Core types (AzDbType, AzContext)
- Backend-specific types (LmdbAzContext, MdbxAzContext)
- AuthorizationContext trait
- Utility functions
- Re-exports
- Thread safety
- Error handling

### 04. Examples
- Basic usage
- Multiple access rights
- Statistics collection
- Caching
- Trace information
- Multi-threaded usage
- Runtime backend selection
- Error handling
- Performance monitoring

### 05. Architecture
- System overview
- Component architecture
- Data flow
- Thread safety
- Performance considerations
- Error handling
- Extension points

### 06. Implementation Details
- LMDB backend implementation
- libmdbx backend implementation
- Authorization logic
- Statistics system
- Global state management
- Storage trait implementation
- Error handling patterns

### 07. Testing
- Test environment setup
- Unit tests
- Integration tests
- Test cleanup
- Benchmark tests
- Debugging tests
- Running tests
- Continuous integration
- Troubleshooting tests

### 08. Troubleshooting
- Database connection errors
- Authorization errors
- Cache issues
- Statistics issues
- Performance issues
- Multi-threading issues
- Compilation issues
- Testing issues
- Logging and debugging
- Getting help

## Additional Resources

### External Documentation
- [v_authorization crate](https://docs.rs/v_authorization/) - Core authorization framework
- [heed documentation](https://docs.rs/heed/) - LMDB Rust bindings
- [libmdbx documentation](https://docs.rs/libmdbx/) - libmdbx Rust bindings
- [LMDB website](http://www.lmdb.tech/) - Lightning Memory-Mapped Database
- [nanomsg website](https://nanomsg.org/) - Scalable messaging library

### Related Projects
- Veda Platform - The complete semantic database and knowledge management system
- v_authorization - Core authorization framework

## Contributing

When adding new documentation:

1. Follow the existing structure
2. Use clear, simple English
3. Provide code examples
4. Update this index
5. Cross-reference related sections

## Version History

- **v0.5.3** - Current version
  - LMDB and libmdbx backend support
  - Optional caching layer
  - Statistics collection
  - Thread-safe operations

## Feedback

For documentation improvements:
- Report issues with unclear sections
- Suggest additional examples
- Request new topics
- Fix errors or typos

