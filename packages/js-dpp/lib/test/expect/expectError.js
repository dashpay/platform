const { expect } = require('chai');

const ValidationResult = require('../../validation/ValidationResult');
const ConsensusError = require('../../errors/ConsensusError');
const JsonSchemaError = require('../../errors/JsonSchemaError');

const expectError = {
  /**
   * @param {ValidationResult} result
   * @param {ConsensusError} [errorClass]
   * @param {number} [count]
   */
  expectValidationError(result, errorClass = ConsensusError, count = 1) {
    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.getErrors()).to.be.lengthOf(count);

    result.getErrors().forEach(error => expect(error).to.be.instanceOf(errorClass));
  },

  /**
   *
   * @param {ValidationResult} result
   * @param [count]
   */
  expectJsonSchemaError(result, count = 1) {
    expectError.expectValidationError(result, JsonSchemaError, count);
  },
};

module.exports = expectError;
