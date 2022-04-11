const path = require('path');

const dotenvSafe = require('dotenv-safe');

dotenvSafe.config({
  path: path.resolve(__dirname, '..', '.env'),
});

const Runner = require('../lib/Runner');

const runner = new Runner();

const benchmarksPath = path.join(__dirname, '..', 'benchmarks', 'index.js');

runner.loadBenchmarks(benchmarksPath);

runner.run(process.env);
