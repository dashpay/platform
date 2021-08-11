const { Transaction } = require('@dashevo/dashcore-lib');

const fetchAssetLockTransactionOutputFactory = require('../../../../../lib/identity/stateTransition/assetLockProof/fetchAssetLockTransactionOutputFactory');
const getChainAssetLockFixture = require('../../../../../lib/test/fixtures/getChainAssetLockProofFixture');
const getInstantAssetLockProofFixture = require('../../../../../lib/test/fixtures/getInstantAssetLockProofFixture');
const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');
const IdentityAssetLockTransactionIsNotFoundError = require('../../../../../lib/errors/IdentityAssetLockTransactionIsNotFoundError');
const UnknownAssetLockProofError = require('../../../../../lib/errors/UnknownAssetLockProofError');

describe('fetchAssetLockTransactionOutputFactory', () => {
  let fetchAssetLockTransactionOutput;
  let stateRepositoryMock;

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    fetchAssetLockTransactionOutput = fetchAssetLockTransactionOutputFactory(stateRepositoryMock);
  });

  describe('InstantAssetLockProof', () => {
    let assetLockProofFixture;

    beforeEach(() => {
      assetLockProofFixture = getInstantAssetLockProofFixture();
    });

    it('should return asset lock output', async () => {
      const assetLockTransactionOutput = await fetchAssetLockTransactionOutput(
        assetLockProofFixture,
      );

      expect(assetLockTransactionOutput).to.deep.equal(assetLockProofFixture.getOutput());
      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
    });
  });

  describe('ChainAssetLockProof', () => {
    let assetLockProofFixture;
    let output;

    beforeEach(() => {
      const rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';
      assetLockProofFixture = getChainAssetLockFixture();
      stateRepositoryMock.fetchTransaction.resolves({
        data: Buffer.from(rawTransaction, 'hex'),
        height: 42,
      });

      const transaction = new Transaction(rawTransaction);
      ([output] = transaction.outputs);
    });

    it('should fetch output from state repository', async () => {
      const assetLockTransactionOutput = await fetchAssetLockTransactionOutput(
        assetLockProofFixture,
      );

      expect(assetLockTransactionOutput).to.deep.equal(output);

      const outPoint = Transaction.parseOutPointBuffer(assetLockProofFixture.getOutPoint());
      const { transactionHash } = outPoint;

      expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(transactionHash);
    });

    it('should throw IdentityAssetLockTransactionIsNotFoundError when transaction is not found', async () => {
      stateRepositoryMock.fetchTransaction.resolves(null);

      try {
        await fetchAssetLockTransactionOutput(
          assetLockProofFixture,
        );

        expect.fail('should throw IdentityAssetLockTransactionIsNotFoundError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(IdentityAssetLockTransactionIsNotFoundError);
        expect(e.getOutPoint()).to.deep.equal(assetLockProofFixture.getOutPoint());
      }
    });
  });

  it('should throw UnknownAssetLockProofError for unknown assetLockProof', async function it() {
    const type = 666;

    const assetLockProofFixture = {
      getType: this.sinonSandbox.stub().returns(type),
    };

    try {
      await fetchAssetLockTransactionOutput(
        assetLockProofFixture,
      );

      expect.fail('should throw UnknownAssetLockProofError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(UnknownAssetLockProofError);
      expect(e.getType()).to.equal(type);
    }
  });
});
