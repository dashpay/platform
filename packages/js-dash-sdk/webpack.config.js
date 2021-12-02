const path = require('path');
const webpackBaseConfig = require("./webpack.base.config");

const webConfig =  {
  ...webpackBaseConfig,
  entry: './build/src/index.js',
  mode: 'production',
  output: {
    path: path.resolve(__dirname, 'dist'),
    libraryTarget: 'umd',
    library: 'Dash',
    filename: 'dash.min.js',
    // fixes ReferenceError: window is not defined
    globalObject: "(typeof self !== 'undefined' ? self : this)"
  },
}

module.exports = [webConfig];
