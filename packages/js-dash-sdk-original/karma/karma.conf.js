const options = require('./options');

module.exports = (config) => {
  config.set({
    ...options,
    logLevel: config.LOG_INFO,
    files: [
      '../src/**/*.spec.ts',
      '../src/test/karma/bootstrap.ts',
    ],
    preprocessors: {
      '../src/**/*.spec.ts': ['webpack'],
      '../src/test/karma/bootstrap.ts': ['webpack'],
    },
  });
};
