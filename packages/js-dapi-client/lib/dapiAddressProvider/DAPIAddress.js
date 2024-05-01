const DAPIAddressHostMissingError = require('./errors/DAPIAddressHostMissingError');

class DAPIAddress {
  /**
   * @param {RawDAPIAddress|DAPIAddress|string} address
   */
  constructor(address) {
    if (address instanceof DAPIAddress) {
      // eslint-disable-next-line no-constructor-return
      return new DAPIAddress(address.toJSON());
    }

    if (typeof address === 'string') {
      const [host, port, ssl] = address.split(':');

      // eslint-disable-next-line no-param-reassign
      address = {
        host,
        port: port ? parseInt(port, 10) : DAPIAddress.DEFAULT_PORT,
        protocol: ssl === 'no-ssl' ? 'http' : DAPIAddress.DEFAULT_PROTOCOL,
        allowSelfSignedCertificate: ssl === 'self-signed',
      };
    }

    if (!address.host) {
      throw new DAPIAddressHostMissingError();
    }

    this.protocol = address.protocol || DAPIAddress.DEFAULT_PROTOCOL;
    this.host = address.host;
    this.port = address.port || DAPIAddress.DEFAULT_PORT;
    this.proRegTxHash = address.proRegTxHash;
    this.allowSelfSignedCertificate = address.allowSelfSignedCertificate || false;

    this.banCount = 0;
    this.banStartTime = undefined;
  }

  /**
   * Get protocol
   * @returns {string}
   */
  getProtocol() {
    return this.protocol;
  }

  /**
   * Get host
   * @returns {string}
   */
  getHost() {
    return this.host;
  }

  /**
   * Set host
   * @param {string} host
   * @returns {DAPIAddress}
   */
  setHost(host) {
    this.host = host;

    return this;
  }

  /**
   * Get port
   * @returns {number}
   */
  getPort() {
    return this.port;
  }

  /**
   * Set port
   * @param {number} port
   * @returns {DAPIAddress}
   */
  setPort(port) {
    this.port = port;

    return this;
  }

  /**
   * Get ProRegTx hash
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
   * @returns {DAPIAddress}
   */
  markAsBanned() {
    this.banCount += 1;
    this.banStartTime = Date.now();

    return this;
  }

  /**
   * Mark address as live
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
   * @returns {RawDAPIAddress}
   */
  toJSON() {
    return {
      protocol: this.getProtocol(),
      host: this.getHost(),
      port: this.getPort(),
      proRegTxHash: this.getProRegTxHash(),
      allowSelfSignedCertificate: this.isSelfSignedCertificateAllowed(),
    };
  }

  toString() {
    return `${this.getProtocol()}://${this.getHost()}:${this.getPort()}`;
  }
}

DAPIAddress.DEFAULT_PORT = 443;
DAPIAddress.DEFAULT_PROTOCOL = 'https';

/**
 * @typedef {object} RawDAPIAddress
 * @property {string} protocol
 * @property {string} host
 * @property {number} [port=443]
 * @property {string} [proRegTxHash]
 * @property {bool} [selfSigned]
 */

module.exports = DAPIAddress;
