const path = require('path');

const commonJSConfig = {
  entry: ['core-js/stable', './lib/DashPlatformProtocol.js'],
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'DashPlatformProtocol.min.js',
    library: 'DashPlatformProtocol',
    libraryTarget: 'umd',
  },
  node: {
    fs: 'empty',
  },
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
