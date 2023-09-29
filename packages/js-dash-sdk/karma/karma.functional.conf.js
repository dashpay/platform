/* eslint-disable import/no-extraneous-dependencies */
const dotenvResult = require('dotenv').config();
const options = require('./options');

if (dotenvResult.error) {
  throw dotenvResult.error;
}

module.exports = (config) => {
  config.set({
    ...options,
    logLevel: config.LOG_INFO,
    files: [
      '../src/test/karma/bootstrap.ts',
      '../tests/functional/sdk.js',
    ],
    preprocessors: {
      '../src/test/karma/bootstrap.ts': ['webpack'],
      '../tests/functional/sdk.js': ['webpack'],
    },
  });
};
