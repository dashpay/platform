/* eslint-disable import/no-extraneous-dependencies */
const webpack = require('webpack');
const dotenvResult = require('dotenv-safe').config();

if (dotenvResult.error) {
  throw dotenvResult.error;
}

module.exports = (config) => {
  config.set({
    frameworks: ['mocha', 'chai'],
    files: [
      'karma.test.loader.js',
      'tests/functional/wallet.js',
    ],
    preprocessors: {
      'karma.test.loader.js': ['webpack'],
      'tests/functional/wallet.js': ['webpack'],
    },
    webpack: {
      mode: 'development',
      optimization: {
        minimize: false,
      },
      plugins: [
        new webpack.EnvironmentPlugin(
          dotenvResult.parsed,
        ),
      ],
      node: {
        fs: 'empty',
      },
    },
    reporters: ['mocha'],
    port: 9876,
    colors: true,
    logLevel: config.LOG_INFO,
    autoWatch: false,
    browsers: ['ChromeHeadless'],
    singleRun: false,
    concurrency: Infinity,
    browserNoActivityTimeout: 7 * 60 * 1000,
    plugins: [
      'karma-mocha',
      'karma-mocha-reporter',
      'karma-chai',
      'karma-chrome-launcher',
      'karma-webpack',
    ],
  });
};
