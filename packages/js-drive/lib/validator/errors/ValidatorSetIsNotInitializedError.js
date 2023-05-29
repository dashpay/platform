const DriveError = require('../../errors/DriveError');

class ValidatorSetIsNotInitializedError extends DriveError {
  constructor() {
    super('Validator Set is not initialized');
  }
}

module.exports = ValidatorSetIsNotInitializedError;
