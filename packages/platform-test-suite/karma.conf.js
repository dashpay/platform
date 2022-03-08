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
    client: {
      mocha: {
        timeout: 650000,
      },
    },
    browserNoActivityTimeout: 900000,
    browserDisconnectTimeout: 900000,
    frameworks: ['mocha', 'chai', 'webpack'],
    files: [
      'lib/test/karma/loader.js',
      './test/functional/core/broadcastTransaction.spec.js',
    ],
    preprocessors: {
      'lib/test/karma/loader.js': ['webpack', 'sourcemap'],
      './test/functional/core/broadcastTransaction.spec.js': ['webpack', 'sourcemap'],
    },
    webpack: {
      mode: 'development',
      devtool: 'inline-source-map',
      plugins: [
        new webpack.ProvidePlugin({
          Buffer: [require.resolve('buffer/'), 'Buffer'],
          process: require.resolve('process/browser'),
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
          events: require.resolve('events/'),
          string_decoder: require.resolve('string_decoder/'),
        },
        extensions: ['.ts', '.js', '.json'],
      },
    },
    reporters: ['mocha'],
    port: 9876,
    colors: true,
    logLevel: config.LOG_INFO,
    browsers: ['Chrome'],
    singleRun: false,
    concurrency: 1,
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
