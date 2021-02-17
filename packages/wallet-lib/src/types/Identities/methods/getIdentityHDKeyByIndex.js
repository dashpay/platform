const ECDSA_KEY_TYPE = 0;
// const BLS_KEY_TYPE = 1;
/**
 * Returns a private key for managing an identity
 * @param {number} identityIndex - Identity index
 * @param {number} keyIndex - keyIndex
 * @return {HDPrivateKey}
 */
function getIdentityHDKeyByIndex(identityIndex, keyIndex) {
  const { keyChain } = this;
  const hardenedFeatureRootKey = keyChain.getHardenedDIP9FeaturePath('HDPrivateKey');

  const identityFeatureKey = hardenedFeatureRootKey.deriveChild(5, true);

  // as defined in https://github.com/dashpay/dips/blob/master/dip-0013.md#identity-authentication-keys
  const identitySubFeatureKey = identityFeatureKey.deriveChild(0, true);

  return identitySubFeatureKey
    .deriveChild(ECDSA_KEY_TYPE, true)
    .deriveChild(identityIndex, true)
    .deriveChild(keyIndex, true);
}

module.exports = getIdentityHDKeyByIndex;
