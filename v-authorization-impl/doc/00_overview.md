# Overview

## Project Description

`v-authorization-impl` is a Rust library providing high-performance database backend implementations for the Veda authorization system. It offers two different storage options: LMDB (via heed) and libmdbx.

## Key Features

- **Dual Backend Support**: Choose between LMDB and libmdbx based on your requirements
- **High Performance**: Optimized for fast authorization checks with optional caching layer
- **Thread-Safe**: Designed for multi-threaded environments with shared database access
- **Statistics Collection**: Optional performance monitoring and statistics reporting
- **Automatic Recovery**: Built-in retry logic for database errors
- **Safe Database Isolation**: Separate paths for different backends to prevent corruption

## Architecture

The library is structured around these main components:

1. **Storage Backends**
   - `LmdbAzContext`: LMDB-based implementation using heed
   - `MdbxAzContext`: libmdbx-based implementation

2. **Unified Interface**
   - `AzContext`: Wrapper that allows runtime selection of backend
   - `AuthorizationContext` trait: Common interface for all implementations

3. **Supporting Components**
   - `StatManager`: Statistics collection and reporting
   - `AuthorizationHelper`: Internal trait for shared authorization logic
   - `Storage`: Trait for database access abstraction

## Use Cases

- Authorization systems requiring high-throughput access control checks
- Multi-tenant applications with complex permission models
- Systems needing persistent authorization data with caching
- Applications requiring detailed authorization statistics

## Database Paths

The library uses separate database paths to prevent accidental data corruption:

- **LMDB Backend**:
  - Main: `./data/acl-indexes/`
  - Cache: `./data/acl-cache-indexes/`

- **libmdbx Backend**:
  - Main: `./data/acl-mdbx-indexes/`
  - Cache: `./data/acl-cache-mdbx-indexes/`

## Version

Current version: 0.5.3

