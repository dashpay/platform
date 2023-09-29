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
      '../tests/functional/**.spec.js',
    ],
    preprocessors: {
      '../tests/functional/**.spec.js': ['webpack', 'sourcemap'],
    },
  });
};
