const createAssetLockProofInstanceJS = require('@dashevo/dpp/lib/identity/stateTransition/assetLockProof/createAssetLockProofInstance');
const getChainAssetLockFixture = require('@dashevo/dpp/lib/test/fixtures/getChainAssetLockProofFixture');
const getInstantAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getInstantAssetLockProofFixture');
const { default: loadWasmDpp } = require('../../../../../dist');

describe('createAssetLockProofInstance', () => {
  let createAssetLockProofInstance;
  let InstantAssetLockProof;
  let ChainAssetLockProof;

  before(async () => {
    ({
      createAssetLockProofInstance,
      InstantAssetLockProof,
      ChainAssetLockProof,
    } = await loadWasmDpp());
  });

  it('should create an instance of InstantAssetLockProof', () => {
    const assetLockProofFixture = getInstantAssetLockProofFixture();
    const instance = createAssetLockProofInstance(assetLockProofFixture.toObject());
    const instanceJS = createAssetLockProofInstanceJS(assetLockProofFixture.toObject());

    expect(instance.toObject()).to.deep.equal(instanceJS.toObject());
    expect(instance).to.be.an.instanceOf(InstantAssetLockProof);
  });

  it('should create an instance of ChainAssetLockProof', () => {
    const assetLockProofFixture = getChainAssetLockFixture();
    const instance = createAssetLockProofInstance(assetLockProofFixture.toObject());
    const instanceJS = createAssetLockProofInstanceJS(assetLockProofFixture.toObject());

    expect(instance.toObject()).to.deep.equal(instanceJS.toObject());
    expect(instance).to.be.an.instanceOf(ChainAssetLockProof);
  });
});
