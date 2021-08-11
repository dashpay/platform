const fetchAssetLockPublicKeyHashFactory = require('../../../../../lib/identity/stateTransition/assetLockProof/fetchAssetLockPublicKeyHashFactory');
const getInstantAssetLockProofFixture = require('../../../../../lib/test/fixtures/getInstantAssetLockProofFixture');
const AssetLockOutputNotFoundError = require('../../../../../lib/identity/errors/AssetLockOutputNotFoundError');

describe('fetchAssetLockPublicKeyHashFactory', () => {
  let fetchAssetLockPublicKeyHash;
  let fetchAssetLockTransactionOutputMock;
  let assetLockProof;

  beforeEach(function beforeEach() {
    fetchAssetLockTransactionOutputMock = this.sinonSandbox.stub();

    fetchAssetLockPublicKeyHash = fetchAssetLockPublicKeyHashFactory(
      fetchAssetLockTransactionOutputMock,
    );

    assetLockProof = getInstantAssetLockProofFixture();
  });

  it('should return public key hash for specified asset lock proof', async () => {
    fetchAssetLockTransactionOutputMock.resolves(assetLockProof.getOutput());

    const result = await fetchAssetLockPublicKeyHash(assetLockProof);

    expect(result).to.deep.equal(assetLockProof.getOutput().script.getData());
  });

  it('should throw AssetLockOutputNotFoundError if output is not found', async () => {
    try {
      await fetchAssetLockPublicKeyHash(assetLockProof);
      expect.fail('should throw AssetLockOutputNotFoundError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(AssetLockOutputNotFoundError);
    }
  });
});
