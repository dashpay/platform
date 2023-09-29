/* eslint-disable import/no-extraneous-dependencies */
const webpack = require('webpack');
const dotenvResult = require('dotenv-safe').config();

const karmaMocha = require('karma-mocha');
const karmaMochaReporter = require('karma-mocha-reporter');
const karmaChai = require('karma-chai');
const karmaChromeLauncher = require('karma-chrome-launcher');
const karmaWebpack = require('karma-webpack');
const webpackBaseConfig = require('../webpack.base.config');

if (dotenvResult.error) {
  throw dotenvResult.error;
}

module.exports = {
  frameworks: ['mocha', 'chai', 'webpack'],
  webpack: {
    ...webpackBaseConfig,
    mode: 'development',
    plugins: [
      ...webpackBaseConfig.plugins,
      new webpack.EnvironmentPlugin(
        dotenvResult.parsed,
      ),
    ],
  },
  reporters: ['mocha'],
  port: 9876,
  colors: true,
  autoWatch: false,
  browsers: ['chromeWithoutSecurity'],
  singleRun: false,
  concurrency: Infinity,
  browserNoActivityTimeout: 7 * 60 * 1000, // 30000 default
  browserDisconnectTimeout: 3 * 2000, // 2000 default
  pingTimeout: 3 * 5000, // 5000 default
  plugins: [
    karmaMocha,
    karmaMochaReporter,
    karmaChai,
    karmaChromeLauncher,
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
