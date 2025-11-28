const options = require('./options.cjs');

module.exports = (config) => {
  config.set({
    ...options,
    files: [
      '../bootstrap.cjs',
      '../unit/**/*.spec.mjs',
    ],
    preprocessors: {
      '../bootstrap.cjs': ['webpack'],
      '../unit/**/*.spec.mjs': ['webpack'],
    },
  });
};
