const path = require('path');

const dotenvSafe = require('dotenv-safe');

const parseDAPISeedsString = require('../lib/client/parseDAPISeedsString');

dotenvSafe.config({
  path: path.resolve(__dirname, '..', '.env'),
});

const Runner = require('../lib/Runner');

const runner = new Runner({
  driveLogPath: process.env.DRIVE_LOG_PATH,
  verbose: process.env.VERBOSE === '1' || process.env.VERBOSE === 'true',
  client: {
    seeds: parseDAPISeedsString(process.env.DAPI_SEED),
    skipSyncBeforeHeight: Number(process.env.SKIP_SYNC_BEFORE_HEIGHT),
    network: process.env.NETWORK,
    faucetPrivateKey: process.env.FAUCET_PRIVATE_KEY,
  },
});

const benchmarksPath = path.join(__dirname, '..', 'benchmarks', 'index.js');

runner.loadBenchmarks(benchmarksPath);

runner.run();
