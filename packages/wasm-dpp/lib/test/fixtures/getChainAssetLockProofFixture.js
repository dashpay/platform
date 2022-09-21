const ChainAssetLockProof = require('../../identity/stateTransition/assetLockProof/chain/ChainAssetLockProof');

function getChainAssetLockProofFixture() {
  const outPoint = {
    outpointHash: '6e200d059fb567ba19e92f5c2dcd3dde522fd4e0a50af223752db16158dabb1d',
    outpointIndex: 0,
  };

  const binaryTransactionHash = Buffer.from(outPoint.outpointHash, 'hex');
  const indexBuffer = Buffer.alloc(4);

  indexBuffer.writeUInt32LE(outPoint.outpointIndex, 0);

  return new ChainAssetLockProof({
    type: 1,
    coreChainLockedHeight: 42,
    outPoint: Buffer.concat([binaryTransactionHash, indexBuffer]),
  });
}

module.exports = getChainAssetLockProofFixture;
