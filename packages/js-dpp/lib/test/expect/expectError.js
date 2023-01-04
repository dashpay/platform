const { expect } = require('chai');

const ValidationResult = require('../../validation/ValidationResult');
const AbstractConsensusError = require('../../errors/consensus/AbstractConsensusError');
const JsonSchemaError = require('../../errors/consensus/basic/JsonSchemaError');

const expectError = {
  /**
   * @param {ValidationResult} result
   * @param {AbstractConsensusError} [errorClass]
   * @param {number} [count]
   * @param [validationResultClass]
   */
  expectValidationError(
    result,
    errorClass = AbstractConsensusError,
    count = 1,
    validationResultClass = ValidationResult,
  ) {
    expect(result).to.be.an.instanceOf(validationResultClass);
    expect(result.getErrors()).to.have.lengthOf(count);

    result.getErrors().forEach((error) => expect(error).to.be.an.instanceOf(errorClass));
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
