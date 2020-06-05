const DAPIAddressHostMissingError = require('./errors/DAPIAddressHostMissingError');

class DAPIAddress {
  /**
   * @param {RawDAPIAddress|DAPIAddress|string} address
   */
  constructor(address) {
    if (address instanceof DAPIAddress) {
      return new DAPIAddress(address.toJSON());
    }

    if (typeof address === 'string') {
      // eslint-disable-next-line no-param-reassign
      address = {
        host: address,
        httpPort: DAPIAddress.DEFAULT_HTTP_PORT,
        grpcPort: DAPIAddress.DEFAULT_GRPC_PORT,
      };
    }

    if (!address.host) {
      throw new DAPIAddressHostMissingError();
    }

    this.host = address.host;
    this.httpPort = address.httpPort || DAPIAddress.DEFAULT_HTTP_PORT;
    this.grpcPort = address.grpcPort || DAPIAddress.DEFAULT_GRPC_PORT;
    this.proRegTxHash = address.proRegTxHash;

    this.banCount = 0;
    this.banStartTime = undefined;
  }

  /**
   * Get host
   *
   * @returns {string}
   */
  getHost() {
    return this.host;
  }

  /**
   * Set host
   *
   * @param {string} host
   * @returns {DAPIAddress}
   */
  setHost(host) {
    this.host = host;

    return this;
  }

  /**
   * Get HTTP port
   *
   * @returns {number}
   */
  getHttpPort() {
    return this.httpPort;
  }

  /**
   * Set HTTP port
   *
   * @param {number} port
   * @returns {DAPIAddress}
   */
  setHttpPort(port) {
    this.httpPort = port;

    return this;
  }

  /**
   * Get gRPC port
   *
   * @returns {number}
   */
  getGrpcPort() {
    return this.grpcPort;
  }

  /**
   * Set gRPC port
   *
   * @param {number} port
   * @returns {DAPIAddress}
   */
  setGrpcPort(port) {
    this.grpcPort = port;

    return this;
  }

  /**
   * Get ProRegTx hash
   *
   * @returns {string}
   */
  getProRegTxHash() {
    return this.proRegTxHash;
  }

  /**
   * @returns {number}
   */
  getBanStartTime() {
    return this.banStartTime;
  }

  /**
   * @returns {number}
   */
  getBanCount() {
    return this.banCount;
  }

  /**
   * Mark address as banned
   *
   * @returns {DAPIAddress}
   */
  markAsBanned() {
    this.banCount += 1;
    this.banStartTime = Date.now();

    return this;
  }

  /**
   * Mark address as live
   *
   * @returns {DAPIAddress}
   */
  markAsLive() {
    this.banCount = 0;
    this.banStartTime = undefined;

    return this;
  }

  /**
   * @returns {boolean}
   */
  isBanned() {
    return this.banCount > 0;
  }

  /**
   * Return DAPIAddress as plain object
   *
   * @returns {RawDAPIAddress}
   */
  toJSON() {
    return {
      host: this.host,
      httpPort: this.httpPort,
      grpcPort: this.grpcPort,
      proRegTxHash: this.proRegTxHash,
    };
  }
}

DAPIAddress.DEFAULT_HTTP_PORT = 3000;
DAPIAddress.DEFAULT_GRPC_PORT = 3010;

/**
 * @typedef {object} RawDAPIAddress
 * @property {string} host
 * @property {number} [httpPort=3000]
 * @property {number} [grpcPort=3010]
 * @property {string} [proRegTxHash]
 */

module.exports = DAPIAddress;
