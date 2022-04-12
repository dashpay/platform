## Bench Suite

> Dash Platform benchmark tool

### Benchmarks

Benchmark configs are located in `benchmarks` directory. New benchmarks can be easily added to `benchmarks/index.js`

At this moment only one type of benchmark is implemented - Documents.

#### Documents

This benchmark publishes a data contract and documents defined in configuration and collects timings from Drive logs.

### Running benchmarks

```bash
yarn setup
yarn start
yarn bench
```

## Maintainer

[@shumkov](https://github.com/shumkov)

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
