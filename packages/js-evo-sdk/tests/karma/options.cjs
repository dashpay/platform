/* eslint-disable import/no-extraneous-dependencies */
const webpack = require('webpack');
const karmaMocha = require('karma-mocha');
const karmaMochaReporter = require('karma-mocha-reporter');
const karmaChai = require('karma-chai');
const karmaChromeLauncher = require('karma-chrome-launcher');
const karmaFirefoxLauncher = require('karma-firefox-launcher');
const karmaWebpack = require('karma-webpack');

module.exports = {
  frameworks: ['mocha', 'chai', 'webpack'],
  webpack: {
    mode: 'development',
    devtool: 'eval',
    module: {
      rules: [
        {
          test: /\.ts$/,
          use: {
            loader: 'ts-loader',
            options: {
              transpileOnly: true,
              compilerOptions: { declaration: false, emitDeclarationOnly: false },
            },
          },
          exclude: /node_modules/,
        },
      ],
    },
    // No special wasm handling needed (WASM is inlined in dist)
    plugins: [
      new webpack.ProvidePlugin({
        Buffer: [require.resolve('buffer/'), 'Buffer'],
        process: require.resolve('process/browser'),
      }),
    ],
    resolve: {
      extensions: ['.mjs', '.js', '.ts'],
      extensionAlias: { '.js': ['.ts', '.js'] },
      alias: {
      },
      fallback: {
        fs: false,
        path: require.resolve('path-browserify'),
        url: require.resolve('url/'),
        util: require.resolve('util/'),
        buffer: require.resolve('buffer/'),
        events: require.resolve('events/'),
        assert: require.resolve('assert/'),
        string_decoder: require.resolve('string_decoder/'),
        process: require.resolve('process/browser'),
      },
    },
  },
  reporters: ['mocha'],
  port: 9876,
  colors: true,
  autoWatch: false,
  browsers: ['ChromeHeadlessInsecure'],
  singleRun: false,
  concurrency: Infinity,
  browserNoActivityTimeout: 7 * 60 * 1000,
  browserDisconnectTimeout: 3 * 2000,
  pingTimeout: 3 * 5000,
  plugins: [
    karmaMocha,
    karmaMochaReporter,
    karmaChai,
    karmaChromeLauncher,
    karmaFirefoxLauncher,
    karmaWebpack,
  ],
  webpackMiddleware: {
    stats: 'errors-warnings',
  },
  customLaunchers: {
    ChromeHeadlessInsecure: {
      base: 'ChromeHeadless',
      flags: [
        '--allow-insecure-localhost',
      ],
      displayName: 'Chrome w/o security',
    },
  },
};
