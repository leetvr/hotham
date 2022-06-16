![Stress Test](logo.png?raw=true)
# Stress Test
This project contains a series of tests designed to *torture* Hotham. It is unfair, unkind and it most certainly does not care what you think of it.

## How to run the tests
Switching between the various tests is accomplished by changing the `test` variable in `src/lib.rs` to some variant of the `StressTest` enum:

```rust
/// Used to select which test to run
pub enum StressTest {
    /// Generate one cube per second
    ManyCubes,
    /// Create a dynamic mesh and increase the number of vertices each second
    ManyVertices,
    /// Load the New Sponza scene into the engine
    Sponza,
}
```

For more information on these tests, and their results, consult the `README.md` file in the parent directory.
