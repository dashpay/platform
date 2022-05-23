const webpack = require('webpack');
const dotenvResult = require('dotenv-safe').config();

const karmaMocha = require('karma-mocha');
const karmaMochaReporter = require('karma-mocha-reporter');
const karmaChai = require('karma-chai');
const karmaChromeLauncher = require('karma-chrome-launcher');
const karmaSourcemapLoader = require('karma-sourcemap-loader');
const karmaWebpack = require('karma-webpack');

const sdkWebpackConfig = require('../js-dash-sdk/webpack.base.config');

if (dotenvResult.error) {
  throw dotenvResult.error;
}

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
      'lib/test/karma/loader.js',
      './test/**/!(proofs|waitForStateTransitionResult).spec.js',
    ],
    preprocessors: {
      'lib/test/karma/loader.js': ['webpack', 'sourcemap'],
      './test/**/!(proofs|waitForStateTransitionResult).spec.js': ['webpack', 'sourcemap'],
    },
    webpack: {
      mode: 'development',
      devtool: 'inline-source-map',
      plugins: [
        ...sdkWebpackConfig.plugins,
        new webpack.EnvironmentPlugin(
          dotenvResult.parsed,
        ),
      ],
      resolve: {
        fallback: sdkWebpackConfig.resolve.fallback,
        extensions: ['.ts', '.js', '.json'],
      },
    },
    reporters: ['mocha'],
    port: 9876,
    colors: true,
    logLevel: config.LOG_INFO,
    browsers: ['ChromeHeadless'],
    singleRun: true,
    concurrency: Infinity,
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
