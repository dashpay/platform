const ValidationError = require('./ValidationError');

class RangeOperatorAllowedOnlyForLastTwoWhereConditionsError extends ValidationError {
  constructor() {
    super('Range operator are allowed only for the last two where conditions');
  }
}

module.exports = RangeOperatorAllowedOnlyForLastTwoWhereConditionsError;
