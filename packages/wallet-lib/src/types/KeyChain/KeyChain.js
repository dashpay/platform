const { Networks, HDPrivateKey, HDPublicKey } = require('@dashevo/dashcore-lib');
const { has } = require('lodash');

// eslint-disable-next-line no-underscore-dangle
const _defaultOpts = {
  network: Networks.testnet.toString(),
  keys: {},
};

class KeyChain {
  constructor(opts = JSON.parse(JSON.stringify(_defaultOpts))) {
    const defaultOpts = JSON.parse(JSON.stringify(_defaultOpts));
    this.network = defaultOpts.network;
    this.keys = { ...defaultOpts.keys };

    if (has(opts, 'HDPrivateKey')) {
      this.type = 'HDPrivateKey';
      this.HDPrivateKey = (typeof opts.HDPrivateKey === 'string') ? HDPrivateKey(opts.HDPrivateKey) : opts.HDPrivateKey;
      this.network = this.HDPrivateKey.network;
    } else if (has(opts, 'HDPublicKey')) {
      this.type = 'HDPublicKey';
      this.HDPublicKey = (typeof opts.HDPublicKey === 'string') ? HDPublicKey(opts.HDPublicKey) : opts.HDPublicKey;
      this.network = this.HDPublicKey.network;
    } else if (has(opts, 'privateKey')) {
      this.type = 'privateKey';
      this.privateKey = opts.privateKey;
    } else if (has(opts, 'publicKey')) {
      this.type = 'publicKey';
      this.publicKey = opts.publicKey;
    } else if (has(opts, 'address')) {
      this.type = 'address';
      this.address = opts.address.toString();
    } else {
      throw new Error('Expect privateKey, publicKey, HDPublicKey, HDPrivateKey or Address');
    }
    if (opts.network) this.network = opts.network;
    if (opts.keys) this.keys = { ...opts.keys };
  }
}

KeyChain.prototype.generateKeyForChild = require('./methods/generateKeyForChild');
KeyChain.prototype.generateKeyForPath = require('./methods/generateKeyForPath');
KeyChain.prototype.getDIP15ExtendedKey = require('./methods/getDIP15ExtendedKey');
KeyChain.prototype.getHardenedBIP44HDKey = require('./methods/getHardenedBIP44HDKey');
KeyChain.prototype.getHardenedDIP9FeatureHDKey = require('./methods/getHardenedDIP9FeatureHDKey');
KeyChain.prototype.getHardenedDIP15AccountKey = require('./methods/getHardenedDIP15AccountKey');
KeyChain.prototype.getKeyForChild = require('./methods/getKeyForChild');
KeyChain.prototype.getKeyForPath = require('./methods/getKeyForPath');
KeyChain.prototype.getPrivateKey = require('./methods/getPrivateKey');
KeyChain.prototype.sign = require('./methods/sign');

module.exports = KeyChain;
