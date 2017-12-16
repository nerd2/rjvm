# Rust JVM [![Build Status](https://travis-ci.org/nerd2/rjvm.svg?branch=master)](https://travis-ci.org/nerd2/rjvm)

This project aims to be a clean implementation of a Java Virtual Machine in Rust. It was initially started by Sam Lancia in order to learn more about Java byte code and to learn Rust.

### Prerequisites

Builts and runs on Ubuntu with openjdk 8 and rust stable

### Installing

Currently the virtual machine is provided only as a library but we plan to implement a "java"-compatible frontend.

## Running the tests

  cargo test

## TODO

- Threading
- GC
- JIT (LibJIT or direct codegen?)

## License

See the [LICENSE.md](LICENSE.md) file for details
