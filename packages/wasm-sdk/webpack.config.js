const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const webpack = require('webpack');
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");
const { exec } = require('child_process');
const fs = require('fs');

module.exports = {
  entry: './index.js',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'bundle.js',
    library: 'DashWasmSDK',
    libraryTarget: 'umd',
    globalObject: 'this'
  },
  plugins: [
    new HtmlWebpackPlugin({
      template: './index.html'
    }),
    new WasmPackPlugin({
      crateDirectory: path.resolve(__dirname, "."),
      outDir: path.resolve(__dirname, "pkg"),
      // Optimize for size
      extraArgs: "--no-typescript -- --features wasm"
    }),
    // Apply wasm-opt optimization after build
    {
      apply: (compiler) => {
        compiler.hooks.afterEmit.tapAsync('WasmOptPlugin', (compilation, callback) => {
          const wasmFile = path.resolve(__dirname, 'pkg', 'wasm_sdk_bg.wasm');
          const optimizedFile = path.resolve(__dirname, 'pkg', 'wasm_sdk_bg_optimized.wasm');
          
          if (fs.existsSync(wasmFile)) {
            exec(`wasm-opt -Oz -o ${optimizedFile} ${wasmFile}`, (error, stdout, stderr) => {
              if (error) {
                console.warn('wasm-opt optimization failed:', error);
                // Don't fail the build if wasm-opt is not available
              } else {
                console.log('WASM optimized successfully');
                // Replace original with optimized version
                try {
                  fs.copyFileSync(optimizedFile, wasmFile);
                  fs.unlinkSync(optimizedFile);
                } catch (e) {
                  console.warn('Failed to replace WASM file:', e);
                }
              }
              callback();
            });
          } else {
            callback();
          }
        });
      }
    },
    // Reduce bundle size by ignoring Node.js modules
    new webpack.IgnorePlugin({
      resourceRegExp: /^(fs|path|crypto|stream|util)$/,
    })
  ],
  module: {
    rules: [
      {
        test: /\.wasm$/,
        type: 'webassembly/async'
      }
    ]
  },
  experiments: {
    asyncWebAssembly: true
  },
  optimization: {
    minimize: true,
    usedExports: true,
    sideEffects: false,
    // Split runtime into separate chunk
    runtimeChunk: 'single',
    splitChunks: {
      chunks: 'all',
      cacheGroups: {
        // Separate vendor modules
        vendor: {
          test: /[\\/]node_modules[\\/]/,
          name: 'vendors',
          priority: 10
        },
        // Separate WASM modules
        wasm: {
          test: /\.wasm$/,
          name: 'wasm',
          priority: 20
        }
      }
    }
  },
  resolve: {
    extensions: ['.js', '.wasm'],
    fallback: {
      // Polyfills for Node.js modules
      "crypto": false,
      "stream": false,
      "path": false,
      "fs": false
    }
  },
  performance: {
    hints: 'warning',
    maxAssetSize: 1024 * 1024, // 1MB
    maxEntrypointSize: 1024 * 1024 // 1MB
  }
};