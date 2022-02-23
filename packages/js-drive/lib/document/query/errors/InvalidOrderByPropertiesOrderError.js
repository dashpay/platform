const ValidationError = require('./ValidationError');

class InvalidOrderByPropertiesOrderError extends ValidationError {
  constructor() {
    super('"orderBy" properties order does not match order of properties in the index');
  }
}

module.exports = InvalidOrderByPropertiesOrderError;
