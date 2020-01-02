# Introduction

`gwasm-api` is a Rust library for interfacing your native apps with [gWasm]. `gWasm` is
Golem's new meta use-case which allows Golem's developers/users to deploy their Wasm
apps on Golem Network.

`gwasm-api` provides convenience structures and functions for creating a `gWasm` task
and connecting with Golem Network all from native Rust code.

In this guide, you will find step-by-step instructions that will get you started with
the API. We will achieve this by building a version of the [g-flite] app albeit without
all the niceties and features such as animated progressbar, etc.

[gWasm]: https://docs.golem.network/#/Products/Brass-Beta/gWASM 
[g-flite]: https://github.com/golemfactory/g-flite
