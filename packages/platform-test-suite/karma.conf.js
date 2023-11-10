const webpack = require('webpack');
const dotenvResult = require('dotenv-safe').config();

const karmaMocha = require('karma-mocha');
const karmaMochaReporter = require('karma-mocha-reporter');
const karmaChai = require('karma-chai');
const karmaChromeLauncher = require('karma-chrome-launcher');
const karmaSourcemapLoader = require('karma-sourcemap-loader');
const karmaWebpack = require('karma-webpack');
const fs = require('fs');
const which = require('which');

if (dotenvResult.error) {
  throw dotenvResult.error;
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
        },
        extensions: ['.ts', '.js', '.json'],
      },
    },
    reporters: ['mocha'],
    port: 9876,
    colors: true,
    logLevel: config.LOG_INFO,
    browsers: ['chromeWithoutSecurity'],
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
    customLaunchers: {
      chromeWithoutSecurity: {
        base: isChromiumExist() ? 'ChromiumHeadless' : 'ChromeHeadless',
        flags: ['--allow-insecure-localhost'],
        displayName: 'Chrome w/o security',
      },
    },
  });
};
