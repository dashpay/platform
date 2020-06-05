const DAPIClientError = require('../../../errors/DAPIClientError');

class WrongHttpCodeError extends DAPIClientError {
  /**
   *
   * @param {object} requestInfo
   * @param {string} requestInfo.host
   * @param {number} requestInfo.port
   * @param {string} requestInfo.method
   * @param {object} requestInfo.params
   * @param {object} requestInfo.options
   * @param {number} statusCode
   * @param {string} statusMessage
   */
  constructor(requestInfo, statusCode, statusMessage) {
    super(`DAPI JSON RPC wrong http code: ${statusMessage}`);

    this.requestInfo = requestInfo;
    this.statusCode = statusCode;
  }

  /**
   * @returns {{host: string, port: number, method: string, params: object, options: object}}
   */
  getRequestInfo() {
    return this.requestInfo;
  }

  /**
   *
   * @returns {number}
   */
  getStatusCode() {
    return this.statusCode;
  }
}

module.exports = WrongHttpCodeError;
