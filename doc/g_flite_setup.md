# Set up Cargo

In this tutorial, we will be building an app, so go ahead and create
a Rust executable by running:

```
cargo new --bin g-flite
```

Next, let us specify some dependencies ahead of time in `Cargo.toml`:

```
# Cargo.toml
[package]
name = "g-flite"
version = "0.1.0"
authors = ["Jakub Konka <jakub.konka@golem.network>"]

[dependencies]
anyhow = "1.0"
appdirs = "0.2"
gwasm-api = "0.1"
hound = { git = "https://github.com/kubkon/hound" }
openssl = "0.10.20"

[features]
openssl_vendored = ["openssl/vendored"]
```
