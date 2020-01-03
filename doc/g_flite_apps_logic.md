# Write the app's logic

The app will consist of 3 bits of functionality which will be executed sequentially:
1. splitting the input text into smaller chunks,
2. packaging each chunk and the `flite` Wasm binary into a `gWasm` task and executing
   it on the Golem Network, and
3. collecting the resultant WAV files and combining them into one final WAV file.

In this tutorial, we will go ahead and implement everything using only the `main.rs`
module. If you were writing a real app (and especially one that is more complicated
than this one, you'd probably want to split the app's functionality into a number of
different modules). We will also specifically not propagate any errors and instead
panic on each (you don't want to do that in your real app).

Fire up your favourite editor and open `main.rs`, and add the basic skeleton for the app:

```rust
use gwasm_api::prelude::*;
use std::{env, fs, process};

fn split_input(text: String) -> Vec<String> {
    unimplemented!()
}

fn run_on_golem(chunks: Vec<String>) -> ComputedTask {
    unimplemented!()
}

fn combine_output(computed_task: ComputedTask) -> Vec<u8> {
    unimplemented!()
}

const FLITE_JS: &[u8] = include_bytes!("../assets/flite.js");
const FLITE_WASM: &[u8] = include_bytes!("../assets/flite.wasm");

fn main() {
    // Read in the input text file
    let filename = if let Some(filename) = env::args().nth(1) {
        filename
    } else {
        eprintln!("No input file specified!");
        process::exit(1);
    }
    let contents = fs::read(&self.input).unwrap();
    let text = String::from_utf8(contents).unwrap();

    // Split the input
    let chunks = split_input(text);

    // Run on Golem using gwasm-api
    let computed_task = run_on_golem(chunks);

    // Combine the output
    let output_wav = combine_output(computed_task);

    // Write to file
    fs::write("output.wav", output_wav).unwrap();
}
```
