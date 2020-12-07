# Compile flite to Wasm

We will not compile [flite] to Wasm ourselves. Instead, we will use already
precompiled binary. Go ahead and download the binary from [here]. You will 
need both the JavaScript file and the Wasm binary:

```
# https://github.com/golemfactory/g-flite/tree/master/assets
..
flite.js
flite.wasm
```

Download them and save them in the `assets` folder:

```
g-flite/
|
|- assets/
|  |
|  |- flite.js
|  |- flite.wasm
|
|- src/
|  |
|  |- main.rs
|
|- Cargo.toml
```

[flite]: http://www.festvox.org/flite/
[here]: https://github.com/golemfactory/g-flite/tree/master/assets
