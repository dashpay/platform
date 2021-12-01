const path = require('path');
const webpack = require('webpack');

const baseConfig = {
  entry: './src/index.ts',
  module: {
    rules: [
      {
        test: /\.ts$/,
        use: 'ts-loader',
        exclude: /node_modules/,
      },
    ],
  },
  resolve: {
    extensions: ['.ts', '.js', '.json'],
    fallback: {
      fs: false,
      util: require.resolve('util/'),
      crypto: require.resolve('crypto-browserify'),
      http: require.resolve('stream-http'),
      https: require.resolve('https-browserify'),
      buffer: require.resolve('buffer/'),
      url: require.resolve('url/'),
      assert: require.resolve('assert/'),
      stream: require.resolve('stream-browserify'),
      path: require.resolve('path-browserify'),
      os: require.resolve('os-browserify/browser'),
      zlib: require.resolve('browserify-zlib'),
      events: require.resolve('events/'),
      string_decoder: require.resolve('string_decoder/'),
    },
  },
  plugins: [
    new webpack.ProvidePlugin({
      Buffer: [require.resolve('buffer/'), 'Buffer'],
      process: require.resolve('process/browser'),
    }),
  ],
  output: {
    path: path.resolve(__dirname, 'dist'),
  },
}
module.exports = baseConfig;
