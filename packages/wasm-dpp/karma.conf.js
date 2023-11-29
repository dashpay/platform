const webpack = require('webpack');
const karmaMocha = require('karma-mocha');
const karmaMochaReporter = require('karma-mocha-reporter');
const karmaChai = require('karma-chai');
const karmaChromeLauncher = require('karma-chrome-launcher');
const karmaFirefoxLauncher = require('karma-firefox-launcher');
const karmaWebpack = require('karma-webpack');

module.exports = (config) => {
  config.set({
    client: {
      mocha: {
        timeout: 5000,
      },
    },
    frameworks: ['mocha', 'chai', 'webpack'],
    files: [
      'lib/test/karma/loader.js',
    ],
    exclude: [
    ],
    preprocessors: {
      'lib/test/karma/loader.js': ['webpack'],
    },
    webpack: {
      entry: './build/lib/index.js',
      mode: 'development',
      optimization: {
        minimize: false,
        moduleIds: 'named',
      },
      plugins: [
        new webpack.ProvidePlugin({
          Buffer: [require.resolve('buffer/'), 'Buffer'],
          process: require.resolve('process/browser'),
        }),
      ],
      resolve: {
        extensions: ['.js'],
        fallback: {
          fs: false,
          ws: false,
          crypto: require.resolve('crypto-browserify'),
          http: require.resolve('stream-http'),
          https: require.resolve('https-browserify'),
          stream: require.resolve('stream-browserify'),
          path: require.resolve('path-browserify'),
          url: require.resolve('url/'),
          util: require.resolve('util/'),
          buffer: require.resolve('buffer/'),
          events: require.resolve('events/'),
          assert: require.resolve('assert/'),
          string_decoder: require.resolve('string_decoder/'),
          process: require.resolve('process/browser'),
        },
      },
    },
    reporters: ['mocha'],
    port: 9876,
    colors: true,
    logLevel: config.LOG_INFO,
    autoWatch: false,
    browsers: [
      'ChromeHeadless',
      'FirefoxHeadless',
    ],
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
  });
};
