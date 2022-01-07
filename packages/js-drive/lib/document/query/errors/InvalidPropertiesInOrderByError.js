const ValidationError = require('./ValidationError');

class InvalidPropertiesInOrderByError extends ValidationError {
  constructor() {
    super('Sorting is allowed only for the last where condition');
  }
}

module.exports = InvalidPropertiesInOrderByError;
