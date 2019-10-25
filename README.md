# gwasm-api [![build-status]][build-link]

[build-status]: https://github.com/golemfactory/gwasm-rust-api/workflows/Continuous%20Integration/badge.svg
[build-link]: https://github.com/golemfactory/gwasm-rust-api/actions

gwasm-api - gWasm API for Rust apps
* [API Documentation (Development)](https://golemfactory.github.io/gwasm-rust-api/gwasm_api/index.html)

[gWasm](https://docs.golem.network/#/Products/Brass-Beta/gWASM) is Golem's new
meta use-case which allows Golem's developers/users to deploy their Wasm apps
on Golem Network. This API providers convenience structures and functions for
creating a gWasm task and connecting with Golem Network all from native Rust code.

## Example

```rust
use gwasm_api::prelude::*;
use failure::Fallible;
use std::path::Path;

struct ProgressTracker;

impl ProgressUpdate for ProgressTracker {
    fn update(&mut self, progress: f64) {
        println!("Current progress = {}", progress);
    }
}

fn main() -> Fallible<()> {
    let binary = GWasmBinary {
        js: &[0u8; 100],   // JavaScript file generated by Emscripten
        wasm: &[0u8; 100], // Wasm binary generated by Emscripten
    };
    let task = TaskBuilder::new("workspace", binary)
        .push_subtask_data(vec![0u8; 100])
        .build()?;
    let computed_task = compute(
        Path::new("datadir"),
        "127.0.0.1",
        61000,
        Net::TestNet,
        task,
        ProgressTracker,
    )?;

    for subtask in computed_task.subtasks {
        for (_, reader) in subtask.data {
            assert!(!reader.buffer().is_empty());
        }
    }

    Ok(())
}
```

## More examples
* [g-flite](https://github.com/golemfactory/g-flite) is a CLI which uses `gwasm-api`
  internally

## License
Licensed under [GPLv3](LICENSE)
