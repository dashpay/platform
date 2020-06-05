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
    super(`DAPI JSON RPC error: ${requestInfo.method} - ${jsonRpcError.message}`);

    this.requestInfo = requestInfo;
    this.errorCode = jsonRpcError.code;
    this.errorMessage = jsonRpcError.message;
    this.errorData = jsonRpcError.data;
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
  getErrorMessage() {
    return this.errorMessage;
  }

  /**
   * Get error data
   *
   * @returns {object}
   */
  getErrorData() {
    return this.errorData;
  }

  /**
   * Get original error code
   *
   * @returns {number}
   */
  getErrorCode() {
    return this.errorCode;
  }
}

module.exports = JsonRpcError;
