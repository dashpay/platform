const AbciError = require('./AbciError');

class UnavailableAbciError extends AbciError {
  constructor() {
    super(
      AbciError.CODES.UNAVAILABLE,
      'The service is currently unavailable',
    );
  }
}

module.exports = UnavailableAbciError;
