const { Script } = require('@dashevo/dashcore-lib');

const getInstantAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getInstantAssetLockProofFixture');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const { default: loadWasmpDpp } = require('../../../../../dist');

describe('fetchAssetLockPublicKeyHashFactory', () => {
  let fetchAssetLockPublicKeyHash;
  let stateRepositoryMock;
  let executionContext;

  let StateTransitionExecutionContext;
  let InstantAssetLockProof;
  let AssetLockOutputNotFoundError;
  let fetchAssetLockPublicKeyHashDPP;

  before(async () => {
    ({
      StateTransitionExecutionContext,
      InstantAssetLockProof,
      AssetLockOutputNotFoundError,
      fetchAssetLockPublicKeyHash: fetchAssetLockPublicKeyHashDPP,
    } = await loadWasmpDpp());
  });

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    executionContext = new StateTransitionExecutionContext();

    fetchAssetLockPublicKeyHash = (proof) => fetchAssetLockPublicKeyHashDPP(
      stateRepositoryMock,
      proof,
      executionContext,
    );
  });

  it('should return public key hash for specified asset lock proof', async () => {
    const assetLockProof = new InstantAssetLockProof(
      getInstantAssetLockProofFixture().toObject(),
    );

    const result = await fetchAssetLockPublicKeyHash(assetLockProof);

    expect(result).to
      .deep.equal(new Script(assetLockProof.getOutput().script).getData());
  });

  it('should throw AssetLockOutputNotFoundError if output is not found', async () => {
    try {
      const assetLockProofJS = getInstantAssetLockProofFixture();
      // Mess up TX outputs
      assetLockProofJS.transaction.outputs = [];

      const assetLockProof = new InstantAssetLockProof(assetLockProofJS.toObject());
      await fetchAssetLockPublicKeyHash(assetLockProof);
      expect.fail('should throw AssetLockOutputNotFoundError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(AssetLockOutputNotFoundError);
    }
  });
});
