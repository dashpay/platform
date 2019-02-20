class DataIsNotAllowedWithActionDeleteError extends Error {
  /**
   * @param {DPObject} dpObject
   */
  constructor(dpObject) {
    super();

    this.dpObject = dpObject;
    this.message = 'Data is not allowed for objects with $action DELETE';

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get DPObject
   *
   * @returns {DPObject}
   */
  getDPObject() {
    return this.dpObject;
  }
}

module.exports = DataIsNotAllowedWithActionDeleteError;
