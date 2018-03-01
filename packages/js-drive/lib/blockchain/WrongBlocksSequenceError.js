// const ExtendableError = require('es6-error');

module.exports = class WrongBlocksSequenceError extends Error {
  constructor(...params) {
    super(...params);

    // Maintains proper stack trace for where our error was thrown
    Error.captureStackTrace(this, WrongBlocksSequenceError);
  }
};
