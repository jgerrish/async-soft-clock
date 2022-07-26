# async-soft-clock
Library for simulating clocks

![build](https://github.com/jgerrish/async-soft-clock/actions/workflows/rust.yml/badge.svg)

# Introduction

This is a library to create and use software clocks.

It allows basic creation of single clocks.

Usage of clocks is simple, just create a clock with a given clock
rate.  Register listeners to receive clock ticks signals and then
start the clock.

The library uses the Tokio asynchronous runtime.  It shows how to
create and use single-threaded runtimes.  The Clock examples abstract
away some of the complexity of the async runtime.  This provides a
simple interface, but at the cost of less flexibility in building
more complex asynchronous functions. 

This crate is not meant for high precision clocks.  The Tokio library
has millisecond precision.

# Examples

To run an example that creates a single clock.

The features shown in this example:
* Single-threaded Tokio runtime
* Create a clock with a shutdown listener with new_with_stopper
* Tick several times and then stop

RUST_LOG=debug cargo run --example single_clock 
