const AbciError = require('./AbciError');

class InsufficientFundsError extends AbciError {
  constructor(balance) {
    super(
      AbciError.CODES.INSUFFICIENT_FUNDS,
      'Not enough credits',
      { balance },
    );
  }
}

module.exports = InsufficientFundsError;
