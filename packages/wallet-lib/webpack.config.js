const path = require('path');

const webConfig = {
  entry: './src/index.js',
  mode: 'production',
  target: 'web',
  node: {
    // Prevent embedded winston to throw error with FS not existing.
    fs: 'empty',
  },
  output: {
    path: path.resolve(__dirname, 'dist'),
    libraryTarget: 'umd',
    filename: 'wallet-lib.min.js',
    // fixes ReferenceError: window is not defined
    globalObject: "(typeof self !== 'undefined' ? self : this)",
  },
  resolve: {
    extensions: ['.ts', '.js', '.json'],
  },
};
module.exports = webConfig;
