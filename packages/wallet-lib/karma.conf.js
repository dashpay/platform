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
      'src/test/karma/loader.js',
      'tests/functional/wallet.js',
    ],
    preprocessors: {
      'src/test/karma/loader.js': ['webpack', 'sourcemap'],
      'tests/functional/wallet.js': ['webpack', 'sourcemap'],
    },
    webpack: {
      mode: 'development',
      devtool: 'inline-source-map',
      plugins: [
        new webpack.EnvironmentPlugin(
          dotenvResult.parsed,
        ),
      ],
      node: {
        // Prevent embedded winston to throw error with FS not existing.
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
    browserNoActivityTimeout: 10 * 60 * 1000,
    plugins: [
      'karma-mocha',
      'karma-mocha-reporter',
      'karma-chai',
      'karma-chrome-launcher',
      'karma-webpack',
      'karma-sourcemap-loader',
    ],
  });
};
