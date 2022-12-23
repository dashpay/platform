const createAssetLockProofInstance = require('@dashevo/dpp/lib/identity/stateTransition/assetLockProof/createAssetLockProofInstance');
const getChainAssetLockFixture = require('@dashevo/dpp/lib/test/fixtures/getChainAssetLockProofFixture');
const getInstantAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getInstantAssetLockProofFixture');
const ChainAssetLockProof = require('@dashevo/dpp/lib/identity/stateTransition/assetLockProof/chain/ChainAssetLockProof');
const InstantAssetLockProof = require('@dashevo/dpp/lib/identity/stateTransition/assetLockProof/instant/InstantAssetLockProof');

describe('createAssetLockProofInstance', () => {
  it('should create an instance of InstantAssetLockProof', () => {
    const assetLockProofFixture = getInstantAssetLockProofFixture();
    const instance = createAssetLockProofInstance(assetLockProofFixture.toObject());

    expect(instance).to.be.an.instanceOf(InstantAssetLockProof);
  });

  it('should create an instance of ChainAssetLockProof', () => {
    const assetLockProofFixture = getChainAssetLockFixture();
    const instance = createAssetLockProofInstance(assetLockProofFixture.toObject());

    expect(instance).to.be.an.instanceOf(ChainAssetLockProof);
  });
});
