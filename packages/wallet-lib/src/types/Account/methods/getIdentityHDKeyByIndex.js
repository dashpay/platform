/**
 * Returns a private key for managing an identity
 * @param {number} identityIndex - Identity index
 * @param {number} keyIndex - keyIndex
 * @return {HDPrivateKey}
 */
function getIdentityHDKeyByIndex(identityIndex, keyIndex) {
  const { keyChain, index: accountIndex } = this;
  const hardenedFeatureRootKey = keyChain.getHardenedDIP9FeaturePath('HDPrivateKey');

  const identityFeatureKey = hardenedFeatureRootKey.deriveChild(5, true);

  return identityFeatureKey
    .deriveChild(accountIndex, true)
    // ECDSA key type
    .deriveChild(0, true)
    .deriveChild(identityIndex, true)
    .deriveChild(keyIndex, true);
}

module.exports = getIdentityHDKeyByIndex;
