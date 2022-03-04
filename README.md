# Libdata

## crates

- datacore
- protocol
- libdata
- libdata-wasm

### datacore

Implementation of `Core` - a secure append-only single-writer log.

### protocol

Replication protocol for sequential logs.

### libdata

Re-exports `datacore` & `protocol`, adds convenient features & wrappers.

### libdata-wasm

Re-exports `libdata` for WASM. Very opinionated.

## RandomAccess\*

- random-access-storage
- random-access-memory
- random-access-disk

### random-access-storage

Interface describing random access read/write.

### random-access-memory

Implementation of `random-access-storage` for in-memory read/write
access.
Mostly useful for testing or ephemeral storage.

### random-access-disk

Implementation of `random-access-storage` for file system backed
storage.
The most basic persistent implementation, no performance optimizations.
All writes are immediately flushed - lower throughput, guaranteed
durability.
