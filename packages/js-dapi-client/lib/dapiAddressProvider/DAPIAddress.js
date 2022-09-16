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
      const [host, httpPort, grpcPort, ssl] = address.split(':');

      // eslint-disable-next-line no-param-reassign
      address = {
        host,
        httpPort: httpPort ? parseInt(httpPort, 10) : DAPIAddress.DEFAULT_HTTP_PORT,
        grpcPort: grpcPort ? parseInt(grpcPort, 10) : DAPIAddress.DEFAULT_GRPC_PORT,
        protocol: ssl === 'no-ssl' ? 'http' : 'https',
        allowSelfSignedCertificate: ssl === 'self-signed',
      };
    }

    if (!address.host) {
      throw new DAPIAddressHostMissingError();
    }

    this.protocol = address.protocol || 'https';
    this.host = address.host;
    this.httpPort = address.httpPort || DAPIAddress.DEFAULT_HTTP_PORT;
    this.grpcPort = address.grpcPort || DAPIAddress.DEFAULT_GRPC_PORT;
    this.proRegTxHash = address.proRegTxHash;
    this.allowSelfSignedCertificate = address.allowSelfSignedCertificate || false;

    this.banCount = 0;
    this.banStartTime = undefined;
  }

  /**
   * Get protocol
   *
   * @returns {string}
   */
  getProtocol() {
    return this.protocol;
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
   * @returns {boolean}
   */
  isSelfSignedCertificateAllowed() {
    return this.allowSelfSignedCertificate;
  }

  /**
   * Return DAPIAddress as plain object
   *
   * @returns {RawDAPIAddress}
   */
  toJSON() {
    return {
      protocol: this.getProtocol(),
      host: this.getHost(),
      httpPort: this.getHttpPort(),
      grpcPort: this.getGrpcPort(),
      proRegTxHash: this.getProRegTxHash(),
      allowSelfSignedCertificate: this.isSelfSignedCertificateAllowed(),
    };
  }

  toString() {
    return `${this.getProtocol()}://${this.getHost()}:${this.getHttpPort()}:${this.getGrpcPort()}`;
  }
}

DAPIAddress.DEFAULT_HTTP_PORT = 3000;
DAPIAddress.DEFAULT_GRPC_PORT = 3010;

/**
 * @typedef {object} RawDAPIAddress
 * @property {string} protocol
 * @property {string} host
 * @property {number} [httpPort=3000]
 * @property {number} [grpcPort=3010]
 * @property {string} [proRegTxHash]
 * @property {bool} [selfSigned]
 */

module.exports = DAPIAddress;
