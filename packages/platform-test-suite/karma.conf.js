const webpack = require('webpack');
const dotenvSafe = require('dotenv-safe');
const path = require('path');

// Set env variables
const { parsed: envs } = dotenvSafe.config({
  path: path.resolve(__dirname, '.env'),
});

module.exports = (config) => {
  config.set({
    client: {
      mocha: {
        timeout: 650000,
      },
    },
    frameworks: ['mocha', 'chai'],
    files: [
      'lib/test/karma/loader.js',
    ],
    preprocessors: {
      'lib/test/karma/loader.js': ['webpack'],
    },
    webpack: {
      mode: 'development',
      resolve: {
        fallback: {
          fs: false,
          http: require.resolve('stream-http'),
          https: require.resolve('https-browserify'),
          crypto: require.resolve('crypto-browserify'),
          buffer: require.resolve('buffer/'),
          assert: require.resolve('assert-browserify'),
          stream: require.resolve('stream-browserify'),
          path: require.resolve('path-browserify'),
          url: require.resolve('url/'),
          os: require.resolve('os-browserify/browser'),
          zlib: require.resolve('browserify-zlib'),
        },
      },
      plugins: [
        new webpack.ProvidePlugin({
          Buffer: ['buffer', 'Buffer'],
          process: 'process/browser',
        }),
        new webpack.EnvironmentPlugin(Object.keys(envs)),
      ],
    },
    reporters: ['mocha'],
    port: 9876,
    colors: true,
    logLevel: config.LOG_INFO,
    autoWatch: false,
    browsers: ['ChromeHeadless', 'FirefoxHeadless'],
    singleRun: false,
    concurrency: Infinity,
    browserNoActivityTimeout: 900000,
    browserDisconnectTimeout: 900000,
    plugins: [
      'karma-mocha',
      'karma-mocha-reporter',
      'karma-chai',
      'karma-chrome-launcher',
      'karma-firefox-launcher',
      'karma-webpack',
    ],
    customLaunchers: {
      FirefoxHeadless: {
        base: 'Firefox',
        flags: ['-headless'],
      },
    },
  });
};
