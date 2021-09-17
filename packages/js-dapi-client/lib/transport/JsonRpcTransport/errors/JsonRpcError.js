const DAPIClientError = require('../../../errors/DAPIClientError');

class JsonRpcError extends DAPIClientError {
  /**
   * @param {object} requestInfo
   * @param {string} requestInfo.host
   * @param {number} requestInfo.port
   * @param {string} requestInfo.method
   * @param {object} requestInfo.params
   * @param {object} requestInfo.options
   * @param {object} jsonRpcError
   * @param {number} jsonRpcError.code
   * @param {string} jsonRpcError.message
   * @param {object} jsonRpcError.data
   */
  constructor(requestInfo, jsonRpcError) {
    super(jsonRpcError.message);

    this.requestInfo = requestInfo;
    this.code = jsonRpcError.code;
    this.data = jsonRpcError.data;
  }

  /**
   * @returns {{host: string, port: number, method: string, params: object, options: object}}
   */
  getRequestInfo() {
    return this.requestInfo;
  }

  /**
   * Get error message
   *
   * @returns {string}
   */
  getMessage() {
    return this.message;
  }

  /**
   * Get error data
   *
   * @returns {object}
   */
  getData() {
    return this.data;
  }

  /**
   * Get original error code
   *
   * @returns {number}
   */
  getCode() {
    return this.code;
  }
}

module.exports = JsonRpcError;
