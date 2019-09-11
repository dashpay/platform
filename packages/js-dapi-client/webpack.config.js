const path = require('path');

const commonJSConfig = {
  entry: ['@babel/polyfill', './src/index.js'],
  module: {
    rules: [
      {
        test: /\.js$/,
        exclude: /(node_modules)/,
        use: {
          loader: 'babel-loader',
          options: {
            presets: ['@babel/preset-env'],
          },
        },
      },
    ],
  },
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'dapi-client.min.js',
    library: 'dapiClient',
    libraryTarget: 'umd',
  },
};

module.exports = [commonJSConfig];
