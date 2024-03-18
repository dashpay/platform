const webpack = require('webpack');
const dotenvResult = require('dotenv-safe').config();
const glob = require('glob');

const karmaMocha = require('karma-mocha');
const karmaMochaReporter = require('karma-mocha-reporter');
const karmaChai = require('karma-chai');
const karmaChromeLauncher = require('karma-chrome-launcher');
const karmaFirefoxLauncher = require('karma-firefox-launcher');
const karmaSourcemapLoader = require('karma-sourcemap-loader');
const karmaWebpack = require('karma-webpack');

if (dotenvResult.error) {
  throw dotenvResult.error;
}

// TODO: Fix test to be running in Browser
const testFilesPattern = './test/**/!(proofs|waitForStateTransitionResult).spec.js';
const processors = ['webpack', 'sourcemap'];
let testFiles = [
  testFilesPattern,
];
let testPreprocessors = {
  [testFilesPattern]: processors,
};

const batchTotal = parseInt(process.env.BROWSER_TEST_BATCH_TOTAL, 10);
const batchIndex = parseInt(process.env.BROWSER_TEST_BATCH_INDEX, 10);

if (batchTotal !== 0) {
  const files = glob.sync(testFilesPattern);
  const batchSize = Math.ceil(files.length / batchTotal);

  const batches = [];
  for (let i = 0; i < files.length; i += batchSize) {
    batches.push(files.slice(i, i + batchSize));
  }

  testFiles = batches[batchIndex] || [];

  testPreprocessors = testFiles.reduce((acc, path) => {
    acc[path] = processors;

    return acc;
  }, {});
}

process.env.FAUCET_ADDRESS = process.env[`FAUCET_${batchIndex + 1}_ADDRESS`];
process.env.FAUCET_PRIVATE_KEY = process.env[`FAUCET_${batchIndex + 1}_PRIVATE_KEY`];

module.exports = (config) => {
  config.set({
    client: {
      mocha: {
        timeout: 650000,
        bail: true,
      },
    },
    browserNoActivityTimeout: 900000,
    browserDisconnectTimeout: 900000,
    frameworks: ['mocha', 'chai', 'webpack'],
    files: [
      'lib/test/karma/bootstrap.js',
      ...testFiles,
    ],
    preprocessors: {
      'lib/test/karma/bootstrap.js': ['webpack', 'sourcemap'],
      ...testPreprocessors,
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
          {
            ...dotenvResult.parsed,
            FAUCET_ADDRESS: process.env.FAUCET_ADDRESS,
            FAUCET_PRIVATE_KEY: process.env.FAUCET_PRIVATE_KEY,
          },
        ),
      ],
      resolve: {
        fallback: {
          fs: false,
          path: false,
          net: false,
          os: false,
          http: false,
          https: false,
          assert: require.resolve('assert/'),
          url: require.resolve('url/'),
          string_decoder: require.resolve('string_decoder/'),
          stream: require.resolve('stream-browserify'),
          buffer: require.resolve('buffer/'),
          crypto: require.resolve('crypto-browserify'),
          events: require.resolve('events/'),
          util: require.resolve('util/'),
          process: require.resolve('process/browser'),
        },
        extensions: ['.ts', '.js', '.json'],
      },
    },
    reporters: ['mocha'],
    port: 9876,
    colors: true,
    logLevel: config.LOG_INFO,
    browsers: ['ChromeHeadlessInsecure'],
    singleRun: true,
    concurrency: Infinity,
    plugins: [
      karmaMocha,
      karmaMochaReporter,
      karmaChai,
      karmaChromeLauncher,
      karmaFirefoxLauncher,
      karmaSourcemapLoader,
      karmaWebpack,
    ],
    customLaunchers: {
      ChromeHeadlessInsecure: {
        base: 'ChromeHeadless',
        flags: ['--allow-insecure-localhost'],
        displayName: 'Chrome w/o security',
      },
    },
  });
};
