const path = require('path');
const webpack = require('webpack');

const webConfig = {
  entry: './src/index.js',
  mode: 'production',
  target: 'web',
  output: {
    path: path.resolve(__dirname, 'dist'),
    libraryTarget: 'umd',
    filename: 'wallet-lib.min.js',
    // fixes ReferenceError: window is not defined
    globalObject: "(typeof self !== 'undefined' ? self : this)",
  },
  resolve: {
    fallback: {
      fs: false,
      crypto: require.resolve('crypto-browserify'),
      buffer: require.resolve('buffer/'),
      assert: require.resolve('assert/'),
      url: require.resolve('url/'),
      path: require.resolve('path-browserify'),
      http: require.resolve('stream-http'),
      https: require.resolve('https-browserify'),
      stream: require.resolve('stream-browserify'),
      util: require.resolve('util/'),
      os: require.resolve('os-browserify/browser'),
      zlib: require.resolve('browserify-zlib'),
      events: require.resolve('events/'),
      string_decoder: require.resolve('string_decoder/'),
      process: require.resolve('process/browser'),
    },
    extensions: ['.ts', '.js', '.json'],
  },
  plugins: [
    new webpack.ProvidePlugin({
      Buffer: [require.resolve('buffer/'), 'Buffer'],
      process: require.resolve('process/browser'),
    }),
  ],
};
module.exports = webConfig;
