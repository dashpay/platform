/* eslint-disable import/no-extraneous-dependencies */
const webpack = require('webpack');
const dotenvResult = require('dotenv-safe').config();

const karmaMocha = require('karma-mocha');
const karmaMochaReporter = require('karma-mocha-reporter');
const karmaChai = require('karma-chai');
const karmaChromeLauncher = require('karma-chrome-launcher');
const karmaSourcemapLoader = require('karma-sourcemap-loader');
const karmaWebpack = require('karma-webpack');

if (dotenvResult.error) {
  throw dotenvResult.error;
}

module.exports = (config) => {
  config.set({
    frameworks: ['mocha', 'chai', 'webpack'],
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
        new webpack.ProvidePlugin({
          Buffer: ['buffer', 'Buffer'],
          process: 'process/browser',
        }),
        new webpack.EnvironmentPlugin(
          dotenvResult.parsed,
        ),
      ],
      resolve: {
        fallback: {
          fs: false,
          crypto: require.resolve('crypto-browserify'),
          buffer: require.resolve('buffer/'),
          assert: require.resolve('assert/'),
          url: require.resolve('url/'),
          path: require.resolve('path-browserify'),
          http: require.resolve('stream-http'),
          https: require.resolve('https-browserify'),
          stream: require.resolve('stream-browserify'),
          util: require.resolve('util/'),
          os: require.resolve('os-browserify/browser'),
          zlib: require.resolve('browserify-zlib'),
        },
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
      karmaMocha,
      karmaMochaReporter,
      karmaChai,
      karmaChromeLauncher,
      karmaSourcemapLoader,
      karmaWebpack,
    ],
  });
};
