class ValidationResult {
  /**
   *
   * @param {boolean} isValid
   * @param {Date} timeWindowStart
   * @param {Date} timeWindowEnd
   */
  constructor(isValid, timeWindowStart, timeWindowEnd) {
    this.valid = isValid;
    this.timeWindowStart = timeWindowStart;
    this.timeWindowEnd = timeWindowEnd;
  }

  /**
   *
   * @returns {Date}
   */
  getTimeWindowStart() {
    return this.timeWindowStart;
  }

  /**
   *
   * @returns {Date}
   */
  getTimeWindowEnd() {
    return this.timeWindowEnd;
  }

  /**
   *
   * @returns {boolean}
   */
  isValid() {
    return this.valid;
  }
}

module.exports = ValidationResult;
