const path = require('path');
const nodeExternals = require('webpack-node-externals');

const baseConfig = require('./webpack.base.config');

const webConfig = Object.assign({}, baseConfig, {
  target: 'web',
  mode: "production",
  output: {
    path: path.resolve(__dirname, 'dist'),
    libraryTarget: 'umd',
    library: 'Dash',
    filename: 'dash.min.js',
    // fixes ReferenceError: window is not defined
    globalObject: "(typeof self !== 'undefined' ? self : this)"
  }
});

const es5Config = Object.assign({}, baseConfig, {
  target: 'node',
  mode: 'development',
  devtool: 'inline-source-map',
  externals: [nodeExternals()],
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'dash.cjs.js',
    libraryTarget: 'commonjs2'
  },
});

module.exports = [webConfig, es5Config];
