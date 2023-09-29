const options = require('./options');

module.exports = (config) => {
  config.set({
    ...options,
    files: [
      '../src/test/karma/loader.js',
    ],
    preprocessors: {
      '../src/test/karma/loader.js': ['webpack', 'sourcemap'],
    },
    logLevel: config.LOG_INFO,
  });
};
