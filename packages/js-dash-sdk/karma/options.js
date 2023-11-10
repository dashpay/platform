/* eslint-disable import/no-extraneous-dependencies */
const webpack = require('webpack');
const dotenvSafe = require('dotenv-safe');
const which = require('which');
const fs = require('fs');

const karmaMocha = require('karma-mocha');
const karmaMochaReporter = require('karma-mocha-reporter');
const karmaChai = require('karma-chai');
const karmaChromeLauncher = require('karma-chrome-launcher');
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

function isChromiumExist() {
  const ChromiumHeadlessBrowser = karmaChromeLauncher['launcher:ChromiumHeadless'][1];
  const chromiumBrowser = new ChromiumHeadlessBrowser(() => { }, {});

  let chromiumPath = chromiumBrowser.DEFAULT_CMD[process.platform];
  if (chromiumBrowser.ENV_CMD && process.env[chromiumBrowser.ENV_CMD]) {
    chromiumPath = process.env[chromiumBrowser.ENV_CMD];
  }

  if (!chromiumPath) {
    return false;
  }

  // On linux, the browsers just return the command, not a path, so we need to check if it exists.
  if (process.platform === 'linux') {
    return !!which.sync(chromiumPath, { nothrow: true });
  }

  return fs.existsSync(chromiumPath);
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
      base: isChromiumExist() ? 'ChromiumHeadless' : 'ChromeHeadless',
      flags: ['--allow-insecure-localhost'],
      displayName: 'Chrome w/o security',
    },
  },
};
