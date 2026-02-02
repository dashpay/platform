const options = require('./options.cjs');

module.exports = (config) => {
  config.set({
    ...options,
    files: [
      // Load bootstrap first to initialize chai and globals
      '../bootstrap.cjs',
      '../functional/**/*.spec.ts',
    ],
    preprocessors: {
      '../bootstrap.cjs': ['webpack'],
      '../functional/**/*.spec.ts': ['webpack'],
    },
  });
};
