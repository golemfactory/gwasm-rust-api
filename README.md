<div align="center">
  <h1><code>gwasm-api</code></h1>

  <p>
    <strong>A Rust library for interfacing your native apps with gWasm.</strong>
  </p>

  <p>
    <a href="https://github.com/golemfactory/gwasm-rust-api/actions"><img src="https://github.com/golemfactory/gwasm-rust-api/workflows/Continuous%20Integration/badge.svg" /></a>
    <a href="https://crates.io/crates/gwasm-api"><img src="https://img.shields.io/crates/v/gwasm-api.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/gwasm-api"><img src="https://img.shields.io/crates/d/gwasm-api.svg?style=flat-square" alt="Download" /></a>
    <a href="https://docs.rs/gwasm-api/"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
  </p>

  <h3>
    <a href="https://golemfactory.github.io/gwasm-rust-api/">Guide</a> 
  </h3>
</div>

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
