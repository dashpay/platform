const ConsensusError = require('./ConsensusError');

class MissingDapObjectTypeError extends ConsensusError {
  constructor() {
    super('$type is not present');
  }
}

module.exports = MissingDapObjectTypeError;
