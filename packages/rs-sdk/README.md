# Dash Platform Rust SDK

This is the official Rust SDK for the Dash Platform. Dash Platform is a Layer 2 cryptocurrency technology that builds upon the Dash layer 1 network. This SDK provides an abstraction layer to simplify usage of the Dash Platform along with data models based on the Dash Platform Protocol (DPP), a CRUD interface, and bindings for other technologies such as C.

## Features

- **Abstraction Layer:** Simplifies the usage of Dash Platform.
- **Data Model:** Based on Dash Platform Protocol (DPP).
- **CRUD Interface:** Allows for easy manipulation of data.
- **Technology Bindings:** Includes bindings for other technologies like C.

## `platform` Module: Data Model

The data model in module `platform` consists of Dash Platform Protocol (DPP) objects that are wrapped into SDK wrapper objects using the Newtype design pattern. These SDK objects have an `Sdk` prefix followed by the name of the DPP object. For example, the wrapper for Identity is `SdkIdentity`.

In addition to this, the `TryFrom` trait has been implemented to allow for easy conversion between SDK and DPP objects.

## `crud` Module: CRUD Interface

The crud module provides a comprehensive interface for Create, Read, Update, and Delete (CRUD) operations on the Dash Platform. The module comprises several traits designed to streamline interactions with the platform:

- Readable: This trait is designed for reading data from the Dash Platform. It requires an object's identifier as an SdkQuery parameter that will return exactly one item.
- Listable: This trait allows listing of data from the Dash Platform. It uses the SdkQuery to define the search criteria for the data to be listed.
- Writable: This trait is currently under development. Once completed, it will enable modification of data on the Dash Platform.

## Testability

TODO

As a developer, I want to execute code that uses rs-sdk without sending actual requests to the server.
I can store requests (test vectors) as dapi_grpc objects, or JSON files on disk.

The same code (but with different setup) should work when connected to the server.

## Error handling

TODO

- use thiserror crate
- prefer returning Error object
- only panic on non-recoverable errors

## Logging

This project uses the `tracing` crate for instrumentation and logging. The `tracing` ecosystem provides a powerful, flexible framework for adding structured, context-aware logs to your program.

To enable logging, you can use the `tracing_subscriber` crate which allows applications to customize how events are processed and recorded.

### Setup

You can setup the logging system as follows:

```rust
pub fn setup_logs() {
    tracing_subscriber::fmt::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::new(
        "info,rs_sdk=trace,h2=info",
    ))
    .pretty()
    .with_ansi(true)
    .try_init()
    .ok();
}
```

In this example, an environment filter is used to control the verbosity of the logs. The filter string "info,rs_sdk=trace,h2=info" indicates that by default, all tracing events at or below the info level will be logged, but for the `rs_sdk` module, trace level events will also be logged. The `h2` module will only log info level events.

The `.pretty()` method configures the subscriber to print events in a human-readable format, and `.with_ansi(true)` enables ANSI color coding. The `try_init()` method initializes the global logger with the constructed subscriber.

Please note that the `setup_logs()` function should be called before any spans or events are created.

For more information on how to use `tracing` and `tracing_subscriber`, please refer to their respective documentation pages.
