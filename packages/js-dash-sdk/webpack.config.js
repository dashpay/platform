const path = require('path');
const webpackBaseConfig = require("./webpack.base.config");

const webConfig =  {
  ...webpackBaseConfig,
  entry: './build/src/index.js',
  devtool: 'source-map',
  mode: 'production',
  target: 'web',
  output: {
    path: path.resolve(__dirname, 'dist'),
    library: {
      name: 'Dash',
      type: 'umd'
    },
    filename: 'dash.min.js',
    // fixes ReferenceError: window is not defined
    globalObject: "(typeof self !== 'undefined' ? self : this)"
  },
}

module.exports = [webConfig];
