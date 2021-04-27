const InstantAssetLockProof = require('./instant/InstantAssetLockProof');
const ChainAssetLockProof = require('./chain/ChainAssetLockProof');

/**
 *
 * @param {RawInstantAssetLockProof|RawChainAssetLockProof} rawAssetLockProof
 * @returns {InstantAssetLockProof|ChainAssetLockProof}
 */
function createAssetLockProofInstance(rawAssetLockProof) {
  const assetLockProofByTypes = {
    [InstantAssetLockProof.type]: InstantAssetLockProof,
    [ChainAssetLockProof.type]: ChainAssetLockProof,
  };

  return new assetLockProofByTypes[rawAssetLockProof.type](rawAssetLockProof);
}

module.exports = createAssetLockProofInstance;
