class InvalidDapObjectStructureError extends Error {
  /**
   * @param {Object[]} errors
   * @param {Object} rawDapObject
   */
  constructor(errors, rawDapObject) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid Dap Object structure';

    this.errors = errors;
    this.rawDapObject = rawDapObject;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get validation errors
   *
   * @return {Object[]}
   */
  getErrors() {
    return this.errors;
  }

  /**
   * Get raw Dap Object
   *
   * @return {Object}
   */
  getRawDapObject() {
    return this.rawDapObject;
  }
}

module.exports = InvalidDapObjectStructureError;
