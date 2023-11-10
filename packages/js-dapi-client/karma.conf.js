const fs = require('fs');
const karmaMocha = require('karma-mocha');
const karmaMochaReporter = require('karma-mocha-reporter');
const karmaChai = require('karma-chai');
const karmaChromeLauncher = require('karma-chrome-launcher');
const karmaFirefoxLauncher = require('karma-firefox-launcher');
const karmaWebpack = require('karma-webpack');
const which = require('which');
const webpackConfig = require('./webpack.config');
// TODO: Use https://github.com/litixsoft/karma-detect-browsers

/**
 * Is chromium exist
 *
 * @return {boolean}
 */
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

module.exports = (config) => {
  config.set({
    frameworks: ['mocha', 'chai', 'webpack'],
    files: [
      'lib/test/karma/loader.js',
    ],
    preprocessors: {
      'lib/test/karma/loader.js': ['webpack'],
    },
    webpack: {
      mode: 'development',
      resolve: webpackConfig[0].resolve,
      plugins: webpackConfig[0].plugins,
    },
    client: {
      mocha: {
        timeout: 5000,
      },
    },
    reporters: ['mocha'],
    port: 9876,
    colors: true,
    logLevel: config.LOG_INFO,
    autoWatch: false,
    browsers: [isChromiumExist() ? 'ChromiumHeadless' : 'ChromeHeadless', 'FirefoxHeadless'],
    singleRun: false,
    concurrency: Infinity,
    plugins: [
      karmaMocha,
      karmaMochaReporter,
      karmaChai,
      karmaChromeLauncher,
      karmaFirefoxLauncher,
      karmaWebpack,
    ],
  });
};
