const path = require('path');
// const BundleAnalyzerPlugin = require('webpack-bundle-analyzer').BundleAnalyzerPlugin;
const webpack = require('webpack');
const nodeExternals = require('webpack-node-externals');

const baseConfig = {
  entry: './src/index.ts',
  // devtool: 'inline-source-map',
  devtool: 'cheap-module-source-map',
  // mode: 'development',
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
  output: {
    path: path.resolve(__dirname, 'dist'),
  },
  resolve: {
    extensions: ['.ts', '.js', '.json'],
  },
  plugins: [
    // new BundleAnalyzerPlugin(),
    new webpack.DefinePlugin({
      'process.env.NODE_ENV': JSON.stringify('production')
    })
  ],
}
const webConfig = Object.assign({}, baseConfig, {
  target: 'web',
  output: {
    ...baseConfig.output,
    libraryTarget: 'umd',
    library: 'DashJS',
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
