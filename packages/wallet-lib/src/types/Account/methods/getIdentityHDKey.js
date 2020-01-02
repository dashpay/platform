const Identity = require('@dashevo/dpp/lib/identity/Identity');

/**
 * Returns a private key for managing an identity
 * @param {number} index - index of an identity
 * @param {string} identityType - type of identity (user, application)
 * @return {PrivateKey}
 */
function getIdentityHDKey(keyIndex = 0, identityType = 'USER') {
  const { keyChain, index: accountIndex } = this;
  const hardenedFeatureRootKey = keyChain.getHardenedDIP9FeaturePath();

  // Feature 5 : identity.
  const identityFeatureKey = hardenedFeatureRootKey.deriveChild(5, true);

  return identityFeatureKey
    .deriveChild(accountIndex, true)
    .deriveChild(Identity.TYPES[identityType.toUpperCase()], false)
    .deriveChild(keyIndex, false);
}

module.exports = getIdentityHDKey;
