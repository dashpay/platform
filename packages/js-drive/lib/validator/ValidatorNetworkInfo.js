class ValidatorNetworkInfo {
  /**
   *
   * @param {string} host
   * @param {number} port
   */
  constructor(host, port) {
    this.host = host;
    this.port = port;
  }

  /**
   * Get validator host
   * @returns {string}
   */
  getHost() {
    return this.host;
  }

  /**
   * Get validator port
   * @returns {number}
   */
  getPort() {
    return this.port;
  }
}

module.exports = ValidatorNetworkInfo;
