
# Dash Platform Strategy Testing Library

## Overview
This library offers a robust framework for developers and testers to simulate a wide range of blockchain activities on Dash Platform through comprehensive testing strategies. Utilizing the `Strategy` struct, it facilitates the simulation of various operations including balance transfers, contract interactions, and identity management with detailed control over state transitions on a per-block basis.

## Key Features
- **State Transition Focused**: Designed to simulate every type of state transition possible on the Dash Platform, providing a comprehensive test environment.
- **Flexible Strategy Design**: Enables the crafting of complex strategies that can include operations like contract registration, document submissions, identity management, and credit transfers across blocks.
- **Dynamic Entity Management**: Allows for the seamless introduction and management of entities such as contracts and identities at any simulation stage without prior setup.
- **Accessible through Dash Platform's UI**: While directly accessible as part of the Dash Platform repository for integration and development, the library can also be conveniently used via the `Strategies` section of the Dash Platform terminal user interface found at `github.com/dashpay/rs-platform-explorer`.

### Usage
Define your testing strategy within your application by instantiating the `Strategy` struct with the desired configurations and operations. For an interactive experience, use the `Strategies` module within the Dash Platform's terminal user interface:

1. Navigate to `github.com/dashpay/rs-platform-explorer`, clone the repo, and run it via `cargo run`.
2. Use the interface to define and manage your strategies.
3. Execute and observe the simulation of blockchain activities as per your defined strategy.

## Example
```rust
let strategy = Strategy {
    contracts_with_updates: vec![...],
    operations: vec![...],
    start_identities: StartIdentities::new(...),
    identities_inserts: Frequency::new(...),
    signer: Some(SimpleSigner::new(...)),
};
```
