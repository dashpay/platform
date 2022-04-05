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

class DerivableKeyChain {
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
DerivableKeyChain.prototype.getForPath = require('./methods/getForPath');
DerivableKeyChain.prototype.getForAddress = require('./methods/getForAddress');
DerivableKeyChain.prototype.getDIP15ExtendedKey = require('./methods/getDIP15ExtendedKey');
DerivableKeyChain.prototype.getFirstUnusedAddress = require('./methods/getFirstUnusedAddress');
DerivableKeyChain.prototype.getHardenedBIP44HDKey = require('./methods/getHardenedBIP44HDKey');
DerivableKeyChain.prototype.getHardenedDIP9FeatureHDKey = require('./methods/getHardenedDIP9FeatureHDKey');
DerivableKeyChain.prototype.getHardenedDIP15AccountKey = require('./methods/getHardenedDIP15AccountKey');
DerivableKeyChain.prototype.getRootKey = require('./methods/getRootKey');
DerivableKeyChain.prototype.getWatchedAddresses = require('./methods/getWatchedAddresses');
DerivableKeyChain.prototype.getIssuedPaths = require('./methods/getIssuedPaths');
DerivableKeyChain.prototype.maybeLookAhead = require('./methods/maybeLookAhead');
DerivableKeyChain.prototype.markAddressAsUsed = require('./methods/markAddressAsUsed');
DerivableKeyChain.prototype.sign = require('./methods/sign');

module.exports = DerivableKeyChain;
