const options = require('./options.cjs');

module.exports = (config) => {
  config.set({
    ...options,
    files: [
      // Load bootstrap first to initialize chai and globals
      '../bootstrap.cjs',
      '../functional/**/*.spec.mjs',
    ],
    preprocessors: {
      '../bootstrap.cjs': ['webpack'],
      '../functional/**/*.spec.mjs': ['webpack'],
    },
  });
};

