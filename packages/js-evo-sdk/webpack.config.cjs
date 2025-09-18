const path = require('path');
const TerserPlugin = require('terser-webpack-plugin');

const base = {
  entry: path.resolve(__dirname, 'src/sdk.ts'),
  mode: 'production',
  target: ['web', 'es2020'],
  devtool: 'source-map',
  module: {
    parser: { javascript: { url: false } },
    rules: [
      { test: /\.ts$/, use: 'ts-loader', exclude: /node_modules/ },
    ],
  },
  resolve: {
    extensions: ['.ts', '.js', '.json'],
    extensionAlias: { '.js': ['.ts', '.js'] },
  },
  optimization: {
    splitChunks: false,
    runtimeChunk: false,
    minimize: true,
    minimizer: [new TerserPlugin({ terserOptions: { keep_classnames: true } })],
  },
};

const esm = {
  ...base,
  experiments: { outputModule: true },
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'sdk.js',
    library: { type: 'module' },
    module: true,
  },
};
module.exports = esm;
