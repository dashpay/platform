const { Networks, HDPrivateKey, HDPublicKey } = require('@dashevo/dashcore-lib');
const { PrivateKey, PublicKey } = require('@dashevo/dashcore-lib');
const { doubleSha256 } = require('../../utils/crypto');
const { mnemonicToHDPrivateKey } = require('../../utils/mnemonic');

function generateKeyChainId(key) {
  const keyChainIdSuffix = doubleSha256(key.toString()).toString('hex').slice(0, 10);
  return `kc${keyChainIdSuffix}`;
}

function fromOptions(opts) {
  let rootKey;
  let rootKeyType;
  let network = Networks.testnet.toString();
  let passphrase = '';

  if (opts) {
    if (opts.passphrase) {
      passphrase = opts.passphrase;
    }
    if (opts.mnemonic) {
      rootKeyType = 'HDPrivateKey';
      rootKey = (typeof opts.mnemonic === 'string') ? HDPrivateKey(opts.HDPrivateKey) : opts.HDPrivateKey;
    }
    if (opts.network) {
      network = opts.network;
    }
    if (opts.HDPrivateKey) {
      rootKeyType = 'HDPrivateKey';
      rootKey = (typeof opts.HDPrivateKey === 'string') ? HDPrivateKey(opts.HDPrivateKey) : opts.HDPrivateKey;
      network = rootKey.network.toString();
    } else if (opts.HDPublicKey) {
      rootKeyType = 'HDPublicKey';
      rootKey = (typeof opts.HDPublicKey === 'string') ? HDPublicKey(opts.HDPublicKey) : opts.HDPublicKey;
      network = rootKey.network.toString();
    } else if (opts.privateKey) {
      rootKeyType = 'privateKey';
      rootKey = (typeof opts.privateKey === 'string') ? new PrivateKey(opts.privateKey, opts.network) : opts.privateKey;
      network = rootKey.network.toString();
    } else if (opts.publicKey) {
      rootKeyType = 'publicKey';
      rootKey = (typeof opts.publicKey === 'string') ? new PublicKey(opts.publicKey, opts.network) : opts.publicKey;
      network = rootKey.network.toString();
    } else if (opts.address) {
      rootKeyType = 'address';
      rootKey = opts.address.toString();
    } else if (opts.mnemonic) {
      return fromOptions({
        ...opts,
        HDPrivateKey: mnemonicToHDPrivateKey(opts.mnemonic, network, passphrase),
      });
    }
  }

  const lookAheadOpts = {
    isWatched: true,
    paths: {},
    ...opts.lookAheadOpts,
  };

  return {
    rootKeyType,
    rootKey,
    network,
    passphrase,
    lookAheadOpts,
  };
}

class KeyChain {
  constructor(opts = {}) {
    const {
      rootKey,
      rootKeyType,
      network,
      lookAheadOpts,
    } = fromOptions(opts);
    if (!rootKeyType || !rootKey) {
      throw new Error('Expect one of [mnemonic, HDPrivateKey, HDPublicKey, privateKey, publicKey, address] to be provided.');
    }
    this.keyChainId = generateKeyChainId(rootKey);

    this.rootKey = rootKey;
    this.network = network;
    this.rootKeyType = rootKeyType;
    this.lookAheadOpts = { isWatched: true, ...lookAheadOpts };

    this.issuedPaths = new Map();

    this.maybeLookAhead();
  }
}
KeyChain.prototype.getForPath = require('./methods/getForPath');
KeyChain.prototype.getForAddress = require('./methods/getForAddress');
KeyChain.prototype.getDIP15ExtendedKey = require('./methods/getDIP15ExtendedKey');
KeyChain.prototype.getFirstUnusedAddress = require('./methods/getFirstUnusedAddress');
KeyChain.prototype.getHardenedBIP44HDKey = require('./methods/getHardenedBIP44HDKey');
KeyChain.prototype.getHardenedDIP9FeatureHDKey = require('./methods/getHardenedDIP9FeatureHDKey');
KeyChain.prototype.getHardenedDIP15AccountKey = require('./methods/getHardenedDIP15AccountKey');
KeyChain.prototype.getRootKey = require('./methods/getRootKey');
KeyChain.prototype.getWatchedAddresses = require('./methods/getWatchedAddresses');
KeyChain.prototype.getIssuedPaths = require('./methods/getIssuedPaths');
KeyChain.prototype.maybeLookAhead = require('./methods/maybeLookAhead');
KeyChain.prototype.markAddressAsUsed = require('./methods/markAddressAsUsed');
KeyChain.prototype.sign = require('./methods/sign');

module.exports = KeyChain;
