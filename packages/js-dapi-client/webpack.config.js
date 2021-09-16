const path = require('path');
const webpack = require('webpack');

const commonJSConfig = {
  entry: ['core-js/stable', './lib/DAPIClient.js'],
  mode: 'production',
  resolve: {
    fallback: {
      fs: false,
      http: false,
      https: false,
      crypto: require.resolve('crypto-browserify'),
      buffer: require.resolve('buffer/'),
      assert: require.resolve('assert-browserify'),
      stream: require.resolve('stream-browserify'),
      path: require.resolve('path-browserify'),
      url: require.resolve('url/'),
    },
  },
  plugins: [
    new webpack.ProvidePlugin({
      Buffer: ['buffer', 'Buffer'],
      process: 'process/browser',
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
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'dapi-client.min.js',
    library: 'DAPIClient',
    libraryTarget: 'umd',
  },
};

module.exports = [commonJSConfig];
