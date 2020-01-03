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
appdirs = "0.2"
gwasm-api = "0.1"
hound = { git = "https://github.com/kubkon/hound" }
openssl = "0.10.20"

[features]
openssl_vendored = ["openssl/vendored"]
```

A few words of explanation are in order here:
* `gwasm-api` refers to this crate.
* We use the [hound] crate to handle concatenating of WAV files
  which we expect after each gWasm subtask completes---note that we use
  my fork of the crate since `flite` seems not to follow the WAV format
  to the letter and hence I've had to introduce some tweaks to be able to
  read and concatenate the resultant WAV files in one final file.
* Even though we don't make use of the [openssl] crate directly, we specify
  it here, so that on Linux and macOS we can rely on prepackaged openssl lib;
  on Windows, you'll need to install it by hand. You can find the [binaries here].

[appdirs]: https://github.com/djc/appdirs-rs
[hound]: https://github.com/kubkon/hound
[openssl]: https://github.com/sfackler/rust-openssl
[binaries here]: https://slproweb.com/products/Win32OpenSSL.html
