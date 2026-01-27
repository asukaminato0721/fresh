# Status and Logging API

## API Reference

### Status and Logging

#### `setStatus`

Display a transient message in the editor's status bar
The message will be shown until the next status update or user action.
Use for feedback on completed operations (e.g., "File saved", "2 matches found").

```typescript
setStatus(message: string): void
```

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `string` | Text to display; keep short (status bar has limited width) |

#### `debug`

Log a debug message from a plugin
Messages appear in log file when running with RUST_LOG=debug.
Useful for plugin development and troubleshooting.

```typescript
debug(message: string): void
```

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `message` | `string` | Debug message; include context like function name and relevant values |
