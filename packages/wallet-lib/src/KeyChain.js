const { PrivateKey, crypto } = require('@dashevo/dashcore-lib');
const { has } = require('lodash');

class KeyChain {
  constructor(opts) {
    if (!opts) throw new Error('Expect some parameters to construct keychain');
    if (has(opts, 'HDRootKey')) {
      this.type = 'HDRootKey';
      this.HDRootKey = opts.HDRootKey;
    } else if (has(opts, 'privateKey')) {
      this.type = 'privateKey';
      this.privateKey = opts.privateKey;
    }
    this.keys = {

    };
  }

  /**
   * Derive from HDRootKey to a specific path
   * @param path
   * @return HDPrivateKey
   */
  generateKeyForPath(path) {
    return (this.type === 'HDRootKey') ? this.HDRootKey.derive(path) : null;
  }

  getPrivateKey() {
    let pk;
    if (this.type === 'HDRootKey') {
      pk = PrivateKey(this.HDRootKey);
    }
    if (this.type === 'privateKey') {
      pk = PrivateKey(this.privateKey);
    }
    return pk;
  }

  /**
   * Get a key from the cache or generate if none
   * @param path
   * @return HDPrivateKey
   */
  getKeyForPath(path) {
    if (!this.keys[path]) {
      if (this.type === 'HDRootKey') {
        this.keys[path] = this.generateKeyForPath(path);
      }
      if (this.type === 'privateKey') {
        this.keys[path] = this.getPrivateKey(path);
      }
    }

    return this.keys[path];
  }

  /**
   * Allow to sign any transaction or a transition object from a valid privateKeys list
   * @param object
   * @param privateKeys
   * @param sigType
   */
  // eslint-disable-next-line class-methods-use-this
  sign(object, privateKeys, sigType = crypto.Signature.SIGHASH_ALL) {
    const handledTypes = ['Transaction', 'SubTxRegistrationPayload'];
    if (!privateKeys) throw new Error('Require one or multiple privateKeys to sign');
    if (!object) throw new Error('Nothing to sign');
    if (!handledTypes.includes(object.constructor.name)) {
      throw new Error(`Unhandled object of type ${object.constructor.name}`);
    }
    const obj = object.sign(privateKeys, sigType);

    if (!obj.isFullySigned()) {
      throw new Error('Not fully signed transaction');
    }
    return obj;
  }
}
module.exports = KeyChain;
