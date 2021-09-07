const AbstractStateError = require('../AbstractStateError');
const Identifier = require('../../../../identifier/Identifier');

class DocumentTimestampWindowViolationError extends AbstractStateError {
  /**
   * @param {string} timestampName
   * @param {Buffer} documentId
   * @param {Date} timestamp
   * @param {Date} timeWindowStart
   * @param {Date} timeWindowEnd
   */
  constructor(timestampName, documentId, timestamp, timeWindowStart, timeWindowEnd) {
    super(`Document ${Identifier.from(documentId)} ${timestampName} timestamp (${timestamp}) are out of block time window from ${timeWindowStart} and ${timeWindowEnd}`);

    this.timestampName = timestampName;
    this.documentId = documentId;
    this.timestamp = timestamp;
    this.timeWindowStart = timeWindowStart;
    this.timeWindowEnd = timeWindowEnd;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get Document timestamp name
   *
   * @return {string}
   */
  getTimestampName() {
    return this.timestampName;
  }

  /**
   * Get Document ID
   *
   * @return {Buffer}
   */
  getDocumentId() {
    return this.documentId;
  }

  /**
   * Get timestamp
   *
   * @return {Date}
   */
  getTimestamp() {
    return this.timestamp;
  }

  /**
   * @returns {Date}
   */
  getTimeWindowStart() {
    return this.timeWindowStart;
  }

  /**
   * @returns {Date}
   */
  getTimeWindowEnd() {
    return this.timeWindowEnd;
  }
}

module.exports = DocumentTimestampWindowViolationError;
