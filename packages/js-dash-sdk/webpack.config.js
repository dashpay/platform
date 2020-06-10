const baseConfig = require('./webpack.base.config');
const nodeExternals = require('webpack-node-externals');

const webConfig = Object.assign({}, baseConfig, {
  target: 'web',
  output: {
    ...baseConfig.output,
    libraryTarget: 'umd',
    library: 'Dash',
    filename: 'dash.min.js',
    // fixes ReferenceError: window is not defined
    globalObject: "(typeof self !== 'undefined' ? self : this)"
  }
});
const es5Config = Object.assign({}, baseConfig, {
  target: 'node',
  // in order to ignore all modules in node_modules folder
  externals: [nodeExternals()],
  output: {
    ...baseConfig.output,
    filename: 'dash.cjs.min.js',
    libraryTarget: 'commonjs2'
  },
});
module.exports = [webConfig, es5Config]
