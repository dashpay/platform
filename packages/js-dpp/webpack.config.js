const path = require('path');

const commonJSConfig = {
  entry: ['@babel/polyfill', './lib/DashApplicationProtocol.js'],
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'DashApplicationProtocol.min.js',
    library: 'dash-platform',
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
