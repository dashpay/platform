const path = require('path');

const commonJSConfig = {
  entry: ['@babel/polyfill', './lib/DashPlatformProtocol.js'],
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'DashPlatformProtocol.min.js',
    library: 'DashPlatformProtocol',
    libraryTarget: 'umd',
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
