# gotodo — a Go to-do CLI

A single-module Go command that keeps a to-do list in one JSON file. No third-party
dependencies.

Because the workspace write policy only permits files inside existing directories,
this module lives flat in `examples/` rather than in its own subdirectory.

## Files

| File | Purpose |
| --- | --- |
| `go.mod` | Module definition (`example.com/gotodo`) |
| `store.go` | `Item`, `List`, and the `Store` type: load, add, edit, complete, remove, clear |
| `main.go` | Command-line entry point and usage text |
| `store_test.go` | Unit tests for the store and ID parsing |

## Usage

```sh
cd examples

go run . add "buy milk"      # added 1: buy milk
go run . add walk dog        # unquoted words are joined
go run . list                # 1 [ ] buy milk
go run . done 1              # done 1
go run . list -a             # -a also shows completed items
go run . undone 1            # undone 1
go run . edit 2 "walk the dog"   # edited 2: walk the dog
go run . rm 1                # removed 1
go run . clear               # removed 1 completed item(s)
go run . help
```

`edit` replaces an item's text and keeps its ID, completion state, and timestamps.
Build a standalone binary with `go build -o gotodo .`.

## Storage

The list is JSON at `$TODO_FILE`, falling back to `~/.gotodo.json`.

```json
{
  "next_id": 3,
  "items": [
    { "id": 1, "text": "buy milk", "done": true, "created_at": "2024-01-01T00:00:00Z",
      "done_at": "2024-01-01T00:00:00Z" },
    { "id": 2, "text": "walk dog", "done": false, "created_at": "2024-01-01T00:00:00Z" }
  ]
}
```

`next_id` is stored separately, so removing an item never lets a later item reuse
its ID. Every mutation writes a temporary file in the target directory, then
renames it over the list (mode `0600`), so a crash cannot truncate the list.

## Tests

```sh
cd examples && go test ./...
```

`agent.yaml` registers this as the named task `gotest` — `argv: [go, -C, examples,
test, ./...]`, a 120-second timeout. Named tasks run no shell and always execute in
the workspace root, so `-C examples` supplies the module directory; a plain
`go test ./examples/...` from the root would fail module resolution.

Note: the repository's `mise run test` / `mise run check` tasks still cover the Rust
workspace only, so `cargo test` says nothing about this module.
