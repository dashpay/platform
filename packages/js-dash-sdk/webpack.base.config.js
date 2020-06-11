const path = require('path');
const baseConfig = {
  entry: './src/index.ts',
  module: {
    rules: [
      {
        test: /\.ts$/,
        use: 'ts-loader',
        exclude: /node_modules/,
      },
    ],
  },
  node: {
    // Prevent embedded winston to throw error with FS not existing.
    fs: 'empty',
  },
  output: {
    path: path.resolve(__dirname, 'dist'),
  },
  resolve: {
    extensions: ['.ts', '.js', '.json'],
    alias: {
      'bn.js': path.resolve(__dirname, 'node_modules', 'bn.js')
    }
  }
}
module.exports = baseConfig;
