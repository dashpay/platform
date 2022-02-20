const DerivableKeyChain = require('../../DerivableKeyChain/DerivableKeyChain');
const logger = require('../../../logger');

function makeChildKeyChainStore(path, opts) {
  logger.debug(`KeyChainStore - make a child keychainstore for ${path}`);
  const masterKeyChain = this.getMasterKeyChain();
  if (!masterKeyChain) throw new Error('Requires a master keychain to be added first.');

  const childKeyChainStore = new this.constructor();
  const keyChainOpts = { network: masterKeyChain.network, ...opts };

  // Accessing the type from getKeyForPath would behave on browser differently due to mangling.
  keyChainOpts[masterKeyChain.rootKeyType] = masterKeyChain.getForPath(path).key;
  const childKeyChain = new DerivableKeyChain(keyChainOpts);
  childKeyChainStore.addKeyChain(childKeyChain, { isMasterKeyChain: true });
  return childKeyChainStore;
}

module.exports = makeChildKeyChainStore;
