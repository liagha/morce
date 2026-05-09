# morce

A lightweight, real-time entity store with WebSocket pub/sub, built in Rust.

morce provides a simple HTTP API for creating, reading, updating, and deleting entities with tag-based metadata. It includes a built-in WebSocket-based pub/sub system for real-time streaming and an interactive terminal console for live exploration and debugging.

## Features

- **HTTP CRUD API** — Create, read, update, and delete entities with tag-based metadata
- **Tag-based queries** — Find entities by exact tag matches or prefix searches
- **Real-time pub/sub** — Subscribe to entity changes via WebSocket with tag filters
- **In-memory store** — Fast, zero-dependency storage with indexed queries
- **Authorization guard** — Built-in permission system using entity-based access control
- **Interactive terminal** — Web-based console with autocomplete, name resolution, and live querying
- **Zero configuration** — Single binary, runs out of the box

## Quick Start

```bash
cargo run
# Server starts on http://127.0.0.1:8080
# Open http://127.0.0.1:8080/console for the interactive terminal
```

## API Overview

### Create an Entity
```bash
curl -X POST http://127.0.0.1:8080/entities \
  -H "x-tags: kind=msg,from=alice,to=bob" \
  -d "Hello, Bob!"
```

### Read an Entity
```bash
curl http://127.0.0.1:8080/entities/{id}
```

### Query Entities
```bash
curl "http://127.0.0.1:8080/entities?kind=msg&from=alice"
```

### Update an Entity
```bash
curl -X PUT http://127.0.0.1:8080/entities/{id} \
  -H "x-tags: kind=msg,from=alice,to=bob,edited=true" \
  -d "Updated message"
```

### Delete an Entity
```bash
curl -X DELETE http://127.0.0.1:8080/entities/{id}
```

### Real-time Subscription (WebSocket)
```bash
websocat ws://127.0.0.1:8080/ws
# Send filter: kind=msg&from=alice
# Receive real-time updates matching the filter
```

## Entity Model

Each entity consists of:
- **id** — UUID v4 automatically assigned
- **load** — Binary payload (any content type)
- **tags** — Key-value metadata for querying and filtering

### Common Tag Patterns

```bash
# Messages
kind=msg, from=alice, to=bob, at=2024-01-01T00:00:00Z

# Users
kind=user, name=alice, email=alice@example.com

# Channels
kind=channel, name=general, created_by=alice

# Permissions
kind=perm, who=user_id, what=resource_id, can=read
```

## Authorization

morce uses entity-based permissions. Create a session entity, then grant permissions:

```bash
# Create a session (simplified - in production use proper auth)
curl -X POST http://127.0.0.1:8080/entities \
  -H "x-tags: kind=session, actor=user_id" \
  -d "session_token"

# Grant permission
curl -X POST http://127.0.0.1:8080/entities \
  -H "x-tags: kind=perm, who=user_id, what=resource_id, can=read"

# Use in requests
curl http://127.0.0.1:8080/entities/{id} \
  -H "Authorization: Bearer session_id"
```

## Console Commands

The built-in terminal console supports:

```bash
create kind=msg,from=alice Hello World     # Create entity
read {id}                                    # Read entity by ID
query kind=msg&from=alice                   # Query by tags
update {id} kind=msg,from=alice Updated     # Update entity
delete {id}                                  # Delete entity
ws                                           # Connect WebSocket
sub kind=msg                                 # Subscribe to filter
help                                         # Show available commands
```

### Console Features
- **Arrow key history** — Navigate through previously executed commands
- **Tab completion** — Autocomplete commands, tag keys, and values
- **@name resolution** — Reference entities by name (@alice resolves to user ID)
- **Syntax highlighting** — Commands and tags are visually distinct
- **File upload** — Drag and drop files for binary payloads

## Architecture

```
morce
├── entity    — Core entity data structure
├── store     — Storage trait and implementations
├── memory    — In-memory store with DashMap
├── index     — Tag-based inverted index
├── hub       — WebSocket pub/sub broker
├── api       — HTTP request handlers
├── ws        — WebSocket handler
├── guard     — Authorization logic
├── parse     — Tag and predicate parsing
├── format    — Entity serialization
└── console   — Web terminal interface
```

## Use Cases

- **Real-time messaging** — Build chat systems with tag-based routing
- **Collaborative editing** — Stream document changes to subscribed clients
- **IoT data store** — Store and query sensor data with metadata
- **Configuration management** — Store and watch configuration changes
- **Event sourcing** — Lightweight event store with replay capability

## Performance

- In-memory storage with lock-free concurrent access (DashMap)
- Indexed queries avoid full scans for indexed tags
- Unbounded WebSocket channels for low-latency delivery
- Zero-copy payload handling with Bytes

## Limitations

- **No persistence** — Data is lost on server restart (by design for this iteration)
- **Single node** — Not distributed (use Redis/RocksDB adapter for production)
- **No authentication providers** — Custom auth must be implemented externally
- **Memory bound** — All data must fit in RAM

## Production Deployment

For production use, consider:
- Adding a persistence layer (RocksDB, SQLite, PostgreSQL)
- Implementing proper authentication (JWT, OAuth2)
- Adding rate limiting and DDoS protection
- Setting up monitoring and alerting
- Running behind a reverse proxy (nginx, Caddy)

## License

MIT

## Contributing

Contributions are welcome! Areas of interest:
- Persistent storage backends
- Additional query operators
- Authentication providers
- Performance optimizations
- Client libraries
