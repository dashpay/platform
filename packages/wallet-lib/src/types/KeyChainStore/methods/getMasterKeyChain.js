function getMasterKeyChain() {
  const keyChainId = this.masterKeyChainId;
  return this.keyChains.get(keyChainId);
}

module.exports = getMasterKeyChain;
