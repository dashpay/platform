/* eslint-disable import/no-extraneous-dependencies */
const webpack = require('webpack');
const dotenvSafe = require('dotenv-safe');
const which = require('which');
const fs = require('fs');

const karmaMocha = require('karma-mocha');
const karmaMochaReporter = require('karma-mocha-reporter');
const karmaChai = require('karma-chai');
const karmaChromeLauncher = require('karma-chrome-launcher');
const karmaSourcemapLoader = require('karma-sourcemap-loader');
const karmaWebpack = require('karma-webpack');

const webpackConfig = require('../webpack.config');

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
  const chromiumBrowser = new ChromiumHeadlessBrowser();

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
      new webpack.EnvironmentPlugin(
        env,
      ),
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
      base: isChromiumExist() ? 'ChromiumHeadless' : 'ChromeHeadless',
      flags: ['--allow-insecure-localhost'],
      displayName: 'Chrome w/o security',
    },
  },
};
