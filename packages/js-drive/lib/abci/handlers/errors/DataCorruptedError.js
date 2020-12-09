const DriveError = require('../../../errors/DriveError');

class DataCorruptedError extends DriveError {
  constructor() {
    super('Data is corrupted. Please reset you node');
  }
}

module.exports = DataCorruptedError;
