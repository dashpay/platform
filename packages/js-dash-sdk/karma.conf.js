const webpackConfig = require("./webpack.base.config");

module.exports = (config) => {
  config.set({
    frameworks: ['mocha', 'chai'],
    files: [
      'src/index.ts',
      'tests/functional/sdk.js',
    ],
    preprocessors: {
      'src/index.ts': ['webpack'],
      'tests/functional/sdk.js': ['webpack'],
    },
    webpack: {...webpackConfig, target:'web'},
    reporters: ['mocha'],
    port: 9876,
    colors: true,
    logLevel: config.LOG_INFO,
    autoWatch: false,
    browsers: ['ChromeHeadless', 'FirefoxHeadless'],
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
    customLaunchers: {
      FirefoxHeadless: {
        base: 'Firefox',
        flags: ['-headless'],
      },
    },
  });
};
