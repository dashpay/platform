const path = require('path');
const webpack = require('webpack');

const commonJSConfig = {
  entry: ['core-js/stable', './lib/DashPlatformProtocol.js'],
  mode: 'production',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'DashPlatformProtocol.min.js',
    library: 'DashPlatformProtocol',
    libraryTarget: 'umd',
  },
  resolve: {
    fallback: {
      fs: false,
      crypto: require.resolve('crypto-browserify'),
      http: require.resolve('stream-http'),
      https: require.resolve('https-browserify'),
      stream: require.resolve('stream-browserify'),
      path: require.resolve('path-browserify'),
      url: require.resolve('url/'),
      util: require.resolve('util/'),
      buffer: require.resolve('buffer/'),
      events: require.resolve('events/'),
      assert: require.resolve('assert/'),
      string_decoder: require.resolve('string_decoder/'),
    },
  },
  plugins: [
    new webpack.ProvidePlugin({
      Buffer: [require.resolve('buffer/'), 'Buffer'],
      process: require.resolve('process/browser'),
    }),
  ],
  module: {
    rules: [
      {
        test: /\.js$/,
        exclude: /(node_modules)/,
        use: {
          loader: 'babel-loader',
        },
      },
    ],
  },
};

module.exports = [commonJSConfig];
