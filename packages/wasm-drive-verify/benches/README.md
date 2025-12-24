# Performance Benchmarks

This directory contains performance benchmarks for the wasm-drive-verify library.

## Running Benchmarks

### Simple Benchmarks

Run the simple timing benchmarks:

```bash
cargo bench --bench simple_benchmarks
```

This will measure the performance of various verification functions with different proof sizes:
- 1KB proofs
- 10KB proofs  
- 100KB proofs

### Benchmark Categories

1. **Identity Verification**: Tests `verify_full_identity_by_identity_id` performance
2. **Document Verification**: Tests `verify_proof` performance
3. **Contract Verification**: Tests `verify_contract` performance
4. **Platform Version Validation**: Tests version validation overhead
5. **Getter Performance**: Tests the efficiency of Vec<u8> to Uint8Array conversions

### Interpreting Results

The benchmarks output timing information showing:
- Total time for N iterations
- Average time per operation

This helps identify:
- Performance bottlenecks
- Scaling characteristics with different proof sizes
- Overhead of type conversions

### Future Improvements

- Add benchmarks for token verification functions
- Add benchmarks for governance verification functions
- Measure memory usage in addition to time
- Add comparative benchmarks with and without optimizations