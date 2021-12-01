const webpack = require('webpack');
const karmaMocha = require('karma-mocha');
const karmaMochaReporter = require('karma-mocha-reporter');
const karmaChai = require('karma-chai');
const karmaChromeLauncher = require('karma-chrome-launcher');
const karmaFirefoxLauncher = require('karma-firefox-launcher');
const karmaWebpack = require('karma-webpack');

module.exports = (config) => {
  config.set({
    frameworks: ['mocha', 'chai', 'webpack'],
    files: [
      'lib/test/karma/loader.js',
    ],
    preprocessors: {
      'lib/test/karma/loader.js': ['webpack'],
    },
    webpack: {
      mode: 'development',
      resolve: {
        fallback: {
          fs: false,
          http: false,
          https: false,
          crypto: require.resolve('crypto-browserify'),
          buffer: require.resolve('buffer/'),
          assert: require.resolve('assert-browserify'),
          stream: require.resolve('stream-browserify'),
          path: require.resolve('path-browserify'),
          url: require.resolve('url/'),
        },
      },
      plugins: [
        new webpack.ProvidePlugin({
          Buffer: ['buffer', 'Buffer'],
          process: 'process/browser',
        }),
      ],
    },
    reporters: ['mocha'],
    port: 9876,
    colors: true,
    logLevel: config.LOG_INFO,
    autoWatch: false,
    browsers: ['ChromeHeadless', 'FirefoxHeadless'],
    singleRun: false,
    concurrency: Infinity,
    plugins: [
      karmaMocha,
      karmaMochaReporter,
      karmaChai,
      karmaChromeLauncher,
      karmaFirefoxLauncher,
      karmaWebpack,
    ],
    customLaunchers: {
      FirefoxHeadless: {
        base: 'Firefox',
        flags: ['-headless'],
      },
    },
  });
};
