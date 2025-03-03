class VersionStatus {
  /**
   * @param {string} dapiVersion - DAPI version
   * @param {string=} driveVersion - Drive ABCI version
   * @param {string=} tenderdashVersion - Tenderdash version
   * @param {number} tenderdashP2pProtocol - Tenderdash Protocol Version
   * @param {number} tenderdashBlockProtocol - Tenderdash Block Version
   * @param {number} driveCurrentProtocol - Current Dash Platform (Drive) protocol version
   * @param {number} driveLatestProtocol - Next Dash Platform (Drive) protocol version
   */
  constructor(
    dapiVersion,
    driveVersion,
    tenderdashVersion,
    tenderdashP2pProtocol,
    tenderdashBlockProtocol,
    driveCurrentProtocol,
    driveLatestProtocol,
  ) {
    this.dapiVersion = dapiVersion;
    this.driveVersion = driveVersion || null;
    this.tenderdashVersion = tenderdashVersion || null;
    this.tenderdashP2pProtocol = tenderdashP2pProtocol;
    this.tenderdashBlockProtocol = tenderdashBlockProtocol;
    this.driveCurrentProtocol = driveCurrentProtocol;
    this.driveLatestProtocol = driveLatestProtocol;
  }

  /**
   * @returns {string|null} DAPI version
   */
  getDapiVersion() {
    return this.dapiVersion;
  }

  /**
   * @returns {string|null} Drive ABCI version
   */
  getDriveVersion() {
    return this.driveVersion;
  }

  /**
   * @returns {string|null} Tenderdash version
   */
  getTenderdashVersion() {
    return this.tenderdashVersion;
  }

  /**
   * @returns {number} Tenderdash P2P protocol
   */
  getTenderdashP2pProtocol() {
    return this.tenderdashP2pProtocol;
  }

  /**
   * @returns {number} Tenderdash Block protocol
   */
  getTenderdashBlockProtocol() {
    return this.tenderdashBlockProtocol;
  }

  /**
   * @returns {number} Drive Current Protocol
   */
  getDriveCurrentProtocol() {
    return this.driveCurrentProtocol;
  }

  /**
   * @returns {number} Drive Latest Protocol
   */
  getDriveLatestProtocol() {
    return this.driveLatestProtocol;
  }
}

module.exports = VersionStatus;
