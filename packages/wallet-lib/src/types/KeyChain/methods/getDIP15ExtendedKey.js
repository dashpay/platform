/**
 * Return the extended  key of the relationship between two dashpay contacts.
 * @param userUniqueId - Current userID
 * @param contactUniqueId - Contact userID
 * @param index - the key index.
 * @param accountIndex[=0] - the internal wallet account from which derivation is done
 * @param type {HDPrivateKey|HDPublicKey} [type=HDPrivateKey] - set the type of returned keys
 * @return {HDPrivateKey|HDPublicKey}
 */
function getDIP15ExtendedKey(userUniqueId, contactUniqueId, index = 0, accountIndex = 0, type = 'HDPrivateKey') {
  if (!['HDPrivateKey', 'HDPublicKey'].includes(this.type)) {
    throw new Error('Wallet is not loaded from a mnemonic or a HDPubKey, impossible to derivate keys');
  }
  if (!userUniqueId || !contactUniqueId) throw new Error('Required userUniqueId and contactUniqueId to be defined');

  // Require a HDPrivateKey for hardened derivation
  const extendedPrivateKey = this
    .getHardenedDIP15AccountKey(accountIndex, 'HDPrivateKey')
    .deriveChild((userUniqueId), true)
    .deriveChild((contactUniqueId), true)
    .deriveChild(index, false);

  return (type === 'HDPublicKey' ? extendedPrivateKey.hdPublicKey : extendedPrivateKey);
}

module.exports = getDIP15ExtendedKey;
