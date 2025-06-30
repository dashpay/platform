/* eslint-disable import/no-extraneous-dependencies */
const webpack = require('webpack');
const dotenvSafe = require('dotenv-safe');

const karmaMocha = require('karma-mocha');
const karmaMochaReporter = require('karma-mocha-reporter');
const karmaChai = require('karma-chai');
const karmaChromeLauncher = require('karma-chrome-launcher');
const karmaFirefoxLauncher = require('karma-firefox-launcher');
const karmaWebpack = require('karma-webpack');
const webpackBaseConfig = require('../webpack.base.config');

let env = {};
if (process.env.LOAD_ENV) {
  const dotenvResult = dotenvSafe.config();
  if (dotenvResult.error) {
    throw dotenvResult.error;
  }
  env = dotenvResult.parsed;
}

module.exports = {
  frameworks: ['mocha', 'chai', 'webpack'],
  webpack: {
    ...webpackBaseConfig,
    mode: 'development',
    plugins: [
      ...webpackBaseConfig.plugins,
      new webpack.EnvironmentPlugin(
        env,
      ),
    ],
  },
  reporters: ['mocha'],
  port: 9876,
  colors: true,
  autoWatch: false,
  browsers: ['ChromeHeadlessInsecure'],
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
    karmaFirefoxLauncher,
    karmaWebpack,
  ],
  customLaunchers: {
    ChromeHeadlessInsecure: {
      base: 'ChromeHeadless',
      flags: ['--allow-insecure-localhost'],
      displayName: 'Chrome w/o security',
    },
  },
};
