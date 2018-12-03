const ConsensusError = require('./ConsensusError');

class JsonSchemaError extends ConsensusError {
  constructor(error) {
    super(error.message);

    Object.assign(this, error);
  }
}

module.exports = JsonSchemaError;
