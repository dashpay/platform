const options = require('./options.cjs');

module.exports = (config) => {
  config.set({
    ...options,
    files: [
      '../bootstrap.cjs',
      '../unit/**/*.spec.ts',
    ],
    preprocessors: {
      '../bootstrap.cjs': ['webpack'],
      '../unit/**/*.spec.ts': ['webpack'],
    },
  });
};
