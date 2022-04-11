const path = require('path');

const dotenvSafe = require('dotenv-safe');

dotenvSafe.config({
  path: path.resolve(__dirname, '..', '.env'),
});

const Runner = require('../lib/Runner');

const runner = new Runner({
  driveLogPath: process.env.DRIVE_LOG_PATH,
  verbose: process.env.VERBOSE === '1' || process.env.VERBOSE === 'true',
});

const benchmarksPath = path.join(__dirname, '..', 'benchmarks', 'index.js');

runner.loadBenchmarks(benchmarksPath);

runner.run();
