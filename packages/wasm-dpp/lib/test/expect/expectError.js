const { expect } = require('chai');

const { default: loadWasmDpp } = require('../../../dist');

const expectError = {
  /**
   * @param {ValidationResult} result
   * @param {AbstractConsensusError} [errorClass]
   * @param {number} [count]
   */
  async expectValidationError(result, errorClass, count = 1) {
    const wasmDpp = await loadWasmDpp();
    if (!errorClass) {
      // eslint-disable-next-line no-param-reassign
      errorClass = wasmDpp.AbstractConsensusError;
    }
    expect(result).to.be.an.instanceOf(wasmDpp.ValidationResult);
    expect(result.getErrors()).to.have.lengthOf(count);

    result.getErrors().forEach((error) => {
      expect(error).to.be.an.instanceOf(errorClass);
    });
  },

  /**
   *
   * @param {ValidationResult} result
   * @param [count]
   */
  async expectJsonSchemaError(result, count = 1) {
    const wasmDpp = await loadWasmDpp();
    await expectError.expectValidationError(result, wasmDpp.JsonSchemaError, count);
  },

  /**
   *
   * @param {ValidationResult} result
   * @param [count]
   */
  async expectValueError(result, count = 1) {
    const wasmDpp = await loadWasmDpp();
    await expectError.expectValidationError(result, wasmDpp.ValueError, count);
  },
};

module.exports = expectError;
