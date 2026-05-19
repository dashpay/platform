import dashcore from '@dashevo/dashcore-lib';
const { Networks, HDPrivateKey, HDPublicKey } = dashcore;
const { PrivateKey, PublicKey } = dashcore;
import { doubleSha256 } from '../../utils/crypto.js';
import { mnemonicToHDPrivateKey } from '../../utils/mnemonic.js';

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
import _DerivableKeyChain_getForPath from './methods/getForPath.js';
DerivableKeyChain.prototype.getForPath = _DerivableKeyChain_getForPath;
import _DerivableKeyChain_getForAddress from './methods/getForAddress.js';
DerivableKeyChain.prototype.getForAddress = _DerivableKeyChain_getForAddress;
import _DerivableKeyChain_getDIP15ExtendedKey from './methods/getDIP15ExtendedKey.js';
DerivableKeyChain.prototype.getDIP15ExtendedKey = _DerivableKeyChain_getDIP15ExtendedKey;
import _DerivableKeyChain_getFirstUnusedAddress from './methods/getFirstUnusedAddress.js';
DerivableKeyChain.prototype.getFirstUnusedAddress = _DerivableKeyChain_getFirstUnusedAddress;
import _DerivableKeyChain_getHardenedBIP44HDKey from './methods/getHardenedBIP44HDKey.js';
DerivableKeyChain.prototype.getHardenedBIP44HDKey = _DerivableKeyChain_getHardenedBIP44HDKey;
import _DerivableKeyChain_getHardenedDIP9FeatureHDKey from './methods/getHardenedDIP9FeatureHDKey.js';
DerivableKeyChain.prototype.getHardenedDIP9FeatureHDKey = _DerivableKeyChain_getHardenedDIP9FeatureHDKey;
import _DerivableKeyChain_getHardenedDIP15AccountKey from './methods/getHardenedDIP15AccountKey.js';
DerivableKeyChain.prototype.getHardenedDIP15AccountKey = _DerivableKeyChain_getHardenedDIP15AccountKey;
import _DerivableKeyChain_getRootKey from './methods/getRootKey.js';
DerivableKeyChain.prototype.getRootKey = _DerivableKeyChain_getRootKey;
import _DerivableKeyChain_getWatchedAddresses from './methods/getWatchedAddresses.js';
DerivableKeyChain.prototype.getWatchedAddresses = _DerivableKeyChain_getWatchedAddresses;
import _DerivableKeyChain_getIssuedPaths from './methods/getIssuedPaths.js';
DerivableKeyChain.prototype.getIssuedPaths = _DerivableKeyChain_getIssuedPaths;
import _DerivableKeyChain_maybeLookAhead from './methods/maybeLookAhead.js';
DerivableKeyChain.prototype.maybeLookAhead = _DerivableKeyChain_maybeLookAhead;
import _DerivableKeyChain_markAddressAsUsed from './methods/markAddressAsUsed.js';
DerivableKeyChain.prototype.markAddressAsUsed = _DerivableKeyChain_markAddressAsUsed;
import _DerivableKeyChain_sign from './methods/sign.js';
DerivableKeyChain.prototype.sign = _DerivableKeyChain_sign;

export default DerivableKeyChain;