class InvalidDPObjectActionError extends Error {
  /**
   * @param {DPObject} dpObject
   */
  constructor(dpObject) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid DP Object action';
    this.dpObject = dpObject;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get DPObject
   *
   * @return {DPObject}
   */
  getDPObject() {
    return this.dpObject;
  }
}

module.exports = InvalidDPObjectActionError;
