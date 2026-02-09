const options = require('./options.cjs');

module.exports = (config) => {
  config.set({
    ...options,
    files: [
      // Load functional bootstrap first (sets TLS override and initializes chai/globals)
      '../functional-bootstrap.cjs',
      '../functional/**/*.spec.ts',
    ],
    preprocessors: {
      '../functional-bootstrap.cjs': ['webpack'],
      '../functional/**/*.spec.ts': ['webpack'],
    },
  });
};
