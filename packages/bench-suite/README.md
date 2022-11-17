## Bench Suite

> Dash Platform benchmark tool

### Benchmarks

Benchmark configs are located in [benchmarks](./benchmarks) directory. New benchmarks can be easily added to [benchmarks/index.js](./benchmarks/index.js)

At this moment two types of benchmark are implemented: documents and function benchmarks.

#### Documents

This benchmark publishes a data contract and documents defined in configuration and collects timings from Drive logs.

### Function

Function benchmark allow to call a function or functions and collect metrics using [performance tools](https://nodejs.org/docs/latest-v16.x/api/perf_hooks.html).

### Running benchmarks

```bash
yarn setup
yarn start
yarn bench
```

## Maintainer

[@shumkov](https://github.com/shumkov)

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashpay/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
