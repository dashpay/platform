const { PrivateKey, Networks, Address } = require('@dashevo/dashcore-lib');
const bs58 = require('bs58');

module.exports = {
  /**
   *
   * @return {Buffer}
   */
  generate() {
    const privateKey = new PrivateKey();
    const publicKey = privateKey.toPublicKey();

    return bs58.decode(publicKey.toAddress(Networks.testnet).toString());
  },
  /**
   *
   * @param {Buffer} buffer
   * @return {boolean}
   */
  validate(buffer) {
    return Address.isValid(bs58.encode(buffer), Networks.testnet, Address.PayToPublicKeyHash);
  },
};
