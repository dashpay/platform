const Runner = require('../lib/Runner');

const runner = new Runner();

runner.loadBenchmarks(__dirname + '/../benchmarks.js');

await runner.run();
