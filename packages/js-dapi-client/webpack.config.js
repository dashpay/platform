const path = require('path');

const commonJSConfig = {
  entry: ['core-js/stable', './lib/DAPIClient.js'],
  mode: 'production',
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
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'dapi-client.min.js',
    library: 'DAPIClient',
    libraryTarget: 'umd',
  },
};

module.exports = [commonJSConfig];
