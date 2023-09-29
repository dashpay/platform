/* eslint-disable import/no-extraneous-dependencies */
const karmaMocha = require('karma-mocha');
const karmaMochaReporter = require('karma-mocha-reporter');
const karmaChai = require('karma-chai');
const karmaChromeLauncher = require('karma-chrome-launcher');
const karmaSourcemapLoader = require('karma-sourcemap-loader');
const karmaWebpack = require('karma-webpack');

const webpackConfig = require('../webpack.config');

module.exports = {
  client: {
    mocha: {
      bail: true,
    },
  },
  frameworks: ['mocha', 'chai', 'webpack'],
  webpack: {
    mode: 'development',
    devtool: 'inline-source-map',
    plugins: [
      ...webpackConfig.plugins,
    ],
    resolve: webpackConfig.resolve,
  },
  reporters: ['mocha'],
  port: 9876,
  colors: true,
  autoWatch: false,
  browsers: ['chromeWithoutSecurity'],
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
  customLaunchers: {
    chromeWithoutSecurity: {
      base: 'ChromeHeadless',
      flags: ['--allow-insecure-localhost'],
      displayName: 'Chrome w/o security',
    },
  },
};
