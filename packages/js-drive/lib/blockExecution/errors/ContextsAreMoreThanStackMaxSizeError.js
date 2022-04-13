const DriveError = require('../../errors/DriveError');

class ContextsAreMoreThanStackMaxSizeError extends DriveError {
  constructor() {
    super('Number of contexts is more than stack max size');
  }
}

module.exports = ContextsAreMoreThanStackMaxSizeError;
