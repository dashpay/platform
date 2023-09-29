const options = require('./options');

module.exports = (config) => {
  config.set({
    ...options,
    logLevel: config.LOG_INFO,
    files: [
      '../tests/functional/**.spec.js',
    ],
    preprocessors: {
      '../tests/functional/**.spec.js': ['webpack', 'sourcemap'],
    },
  });
};
