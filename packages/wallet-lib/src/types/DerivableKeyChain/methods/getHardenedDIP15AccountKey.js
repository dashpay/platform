/**
 * Return a safier root path to derivate from
 * @param {number} [accountIndex=0] - set the account index
 * @param {HDPrivateKey|HDPublicKey} [type=HDPrivateKey] - set the type of returned keys
 * @return {HDPrivateKey|HDPublicKey}
 */
function getHardenedDIP15AccountKey(accountIndex = 0, type = 'HDPrivateKey') {
  const hardenedFeatureRootKey = this.getHardenedDIP9FeatureHDKey(type);

  // Feature is set to 15' for all DashPay Incoming Funds derivation paths (see DIP15).
  const featureKey = hardenedFeatureRootKey.deriveChild(15, true);
  return featureKey.deriveChild(accountIndex, true);
}
module.exports = getHardenedDIP15AccountKey;
