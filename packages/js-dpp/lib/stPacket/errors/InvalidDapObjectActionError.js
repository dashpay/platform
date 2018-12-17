class InvalidDapObjectActionError extends Error {
  /**
   * @param {DapObject} dapObject
   */
  constructor(dapObject) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid DAP Object action';
    this.dapObject = dapObject;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get Dap Object
   *
   * @return {DapObject}
   */
  getDapObject() {
    return this.dapObject;
  }
}

module.exports = InvalidDapObjectActionError;
