function addKeyChain(keychain, opts = {}) {
  if (this.keyChains.has(keychain.keyChainId)) {
    throw new Error(`Trying to add already existing keyChain ${keychain.keyChainId}`);
  }

  this.keyChains.set(keychain.keyChainId, keychain);

  if (opts) {
    if (opts.isMasterKeyChain && !this.masterKeyChainId) {
      this.masterKeyChainId = keychain.keyChainId;
    }
  }
}

module.exports = addKeyChain;
