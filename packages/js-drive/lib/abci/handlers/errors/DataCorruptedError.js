const DriveError = require('../../../errors/DriveError');

class DataCorruptedError extends DriveError {
  constructor(e) {
    super('Cant\' commit previous block. Please reset you node');

    this.error = e;
  }
}

module.exports = DataCorruptedError;
