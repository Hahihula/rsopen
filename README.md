# rsopen

`rsopen` is a multiplatform application launcher written in Rust. It attempts to launch an application by name using the following strategy:

1.  **Native Launch**: Uses the OS native command (e.g., `open`, `start`) if the app is in your PATH or registered.
2.  **Desktop Entry Search (Linux)**: Searches standard desktop entry locations (e.g., `/usr/share/applications`) for applications matching the name (exact or fuzzy) and parses the `Exec` command.
3.  **Common Path Search**: Searches standard installation directories (e.g., `/Applications`, `C:\Program Files`, `/usr/bin`) for executables.
4.  **Full Search**: Recursively searches the filesystem (with optimization/filtering) for the best match.

## Features

- **Multiplatform**: Supports Windows, macOS, and Linux.
- **Fuzzy Search**: Uses Levenshtein distance to find the closest match if an exact name isn't found.
- **Robust Searching**: Filters out directories and handles complex launch commands (e.g., in `.desktop` files).

## Usage

### As a CLI

```bash
# Launch generic application
rsopen firefox

# Launch with verbose output (to see search process)
rsopen -v "Terminal"
```

### As a Library

```rust
use rsopen::launch_app;

fn main() {
    // Launch 'firefox' with verbose=false
    if let Err(e) = launch_app("firefox", false) {
        eprintln!("Error: {}", e);
    }
}
```

## Installation

```bash
cargo install rsopen
```
