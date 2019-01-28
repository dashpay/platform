module.exports = (config) => {
  config.set({
    basePath: '',
    frameworks: ['mocha', 'chai'],
    files: [
      './test/dist/bundle.js',
      './test.spec.js',
    ],
    exclude: [
    ],
    preprocessors: {
      './test/dist/bundle.js': ['webpack'],
      './test.spec.js': ['webpack'],
    },
    webpack: {
      mode: 'production',
      node: {
        fs: 'empty',
      },
      module: {
        rules: [
          {
            test: /dist\/.*\.js$/,
            exclude: /(node_modules)/,
          },
          {
            test: /src\/.*\.js$/,
            exclude: /(node_modules)/,
            use: {
              loader: 'babel-loader',
              options: {
                presets: ['@babel/preset-env'],
              },
            },
          },
        ],
      },
    },
    reporters: ['mocha'],
    port: 9876,
    colors: true,
    logLevel: config.LOG_INFO,
    autoWatch: false,
    browsers: ['Chrome', 'Firefox'],
    singleRun: false,
    concurrency: Infinity,
    plugins: [
      'karma-mocha',
      'karma-mocha-reporter',
      'karma-chai',
      'karma-chrome-launcher',
      'karma-firefox-launcher',
      'karma-webpack',
    ],
  });
};
