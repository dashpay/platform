const path = require('path');
const webpack = require('webpack');
const baseConfig = {
  entry: './src/index.ts',
  // devtool: 'inline-source-map',
  devtool: 'cheap-module-source-map',
  //mode: 'development',
  mode: "production",
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
  }
}
module.exports = baseConfig;
