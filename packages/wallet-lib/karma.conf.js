/* eslint-disable import/no-extraneous-dependencies */
const webpack = require('webpack');
const dotenvResult = require('dotenv-safe').config();

const karmaMocha = require('karma-mocha');
const karmaMochaReporter = require('karma-mocha-reporter');
const karmaChai = require('karma-chai');
const karmaChromeLauncher = require('karma-chrome-launcher');
const karmaSourcemapLoader = require('karma-sourcemap-loader');
const karmaWebpack = require('karma-webpack');

const webpackConfig = require('./webpack.config');

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
        ...webpackConfig.plugins,
        new webpack.EnvironmentPlugin(
          dotenvResult.parsed,
        ),
      ],
      resolve: webpackConfig.resolve,
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
