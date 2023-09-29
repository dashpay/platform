const dotenvResult = require('dotenv-safe').config();
const options = require('./options');

if (dotenvResult.error) {
  throw dotenvResult.error;
}

module.exports = (config) => {
  config.set({
    ...options,
    files: [
      '../src/test/karma/loader.js',
    ],
    preprocessors: {
      '../src/test/karma/loader.js': ['webpack', 'sourcemap'],
    },
    logLevel: config.LOG_INFO,
  });
};
