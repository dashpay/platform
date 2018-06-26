const DashCore = require('@dashevo/dashcore-lib');

class KeyChain {
  constructor(HDKey, derivationPath = 'm/1') {
    this.derivationPath = derivationPath;
    this.HDKey = HDKey || new DashCore.HDPrivateKey();
  }

  /**
   * Return newly derived private key
   * @return {string}
   */
  getNewPrivateKey() {
    return this.HDKey.derive(this.derivationPath).toString();
  }
}

module.exports = KeyChain;
