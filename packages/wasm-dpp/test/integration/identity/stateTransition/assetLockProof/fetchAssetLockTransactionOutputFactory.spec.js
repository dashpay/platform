const { Transaction } = require('@dashevo/dashcore-lib');

const getChainAssetLockFixture = require('../../../../../lib/test/fixtures/getChainAssetLockProofFixture');
const getInstantAssetLockProofFixture = require('../../../../../lib/test/fixtures/getInstantAssetLockProofFixture');
const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');
const { default: loadWasmDpp } = require('../../../../../dist');

describe.skip('fetchAssetLockTransactionOutputFactory', () => {
  let fetchAssetLockTransactionOutput;
  let stateRepositoryMock;
  let executionContext;

  let StateTransitionExecutionContext;
  let ChainAssetLockProof;
  let AssetLockTransactionIsNotFoundError;
  let UnknownAssetLockProofTypeError;

  let fetchAssetLockTransactionOutputDPP;

  before(async () => {
    ({
      StateTransitionExecutionContext,
      fetchAssetLockTransactionOutput: fetchAssetLockTransactionOutputDPP,
      ChainAssetLockProof,
      AssetLockTransactionIsNotFoundError,
      UnknownAssetLockProofTypeError,
    } = await loadWasmDpp());
  });

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinon);

    executionContext = new StateTransitionExecutionContext();

    fetchAssetLockTransactionOutput = (proof, context) => fetchAssetLockTransactionOutputDPP(
      stateRepositoryMock,
      proof,
      context,
    );
  });

  describe('InstantAssetLockProof', () => {
    let assetLockProofFixture;

    beforeEach(async () => {
      assetLockProofFixture = await getInstantAssetLockProofFixture();
    });

    it('should return asset lock output', async () => {
      const assetLockTransactionOutput = await fetchAssetLockTransactionOutput(
        assetLockProofFixture,
        executionContext,
      );

      expect(assetLockTransactionOutput).to.deep.equal(assetLockProofFixture.getOutput());
      expect(stateRepositoryMock.fetchTransaction).to.not.be.called();
    });
  });

  describe('ChainAssetLockProof', () => {
    let assetLockProofFixture;
    let output;
    let transactionHash;

    beforeEach(() => {
      const rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';
      const assetLockProofFixtureJS = getChainAssetLockFixture();

      const outPoint = Transaction
        .parseOutPointBuffer(Buffer.from(assetLockProofFixtureJS.getOutPoint()));
      ({ transactionHash } = outPoint);

      const rawProof = assetLockProofFixtureJS.toObject();

      // Change endianness of raw txId bytes in outPoint to match expectation of dashcore-rust
      const txId = rawProof.outPoint.slice(0, 32);
      const outputIndex = rawProof.outPoint.slice(32);
      txId.reverse();
      rawProof.outPoint = Buffer.concat([txId, outputIndex]);

      assetLockProofFixture = new ChainAssetLockProof(rawProof);

      stateRepositoryMock.fetchTransaction.resolves({
        data: Buffer.from(rawTransaction, 'hex'),
        height: 42,
      });

      const transaction = new Transaction(rawTransaction);
      ([output] = transaction.outputs);
    });

    it('should fetch output from state repository', async function () {
      const assetLockTransactionOutput = await fetchAssetLockTransactionOutput(
        assetLockProofFixture,
        executionContext,
      );

      expect(assetLockTransactionOutput).to.deep.equal(output.toObject());

      expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(
        transactionHash,
        this.sinon.match.instanceOf(StateTransitionExecutionContext),
      );
    });

    it('should throw AssetLockTransactionIsNotFoundError when transaction is not found', async () => {
      stateRepositoryMock.fetchTransaction.resolves(null);

      try {
        await fetchAssetLockTransactionOutput(
          assetLockProofFixture,
          executionContext,
        );

        expect.fail('should throw AssetLockTransactionIsNotFoundError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(AssetLockTransactionIsNotFoundError);
        expect(e.getTransactionId()).to.deep.equal(transactionHash);
      }
    });

    it('should return mocked output on dry run', async function shouldReturn() {
      executionContext.enableDryRun();

      const result = await fetchAssetLockTransactionOutput(
        assetLockProofFixture,
        executionContext,
      );

      executionContext.disableDryRun();

      expect(result).to.deep.equal({
        satoshis: 1000,
        script: '',
      });

      expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(
        transactionHash,
        this.sinon.match.instanceOf(StateTransitionExecutionContext),
      );
    });
  });

  it('should throw UnknownAssetLockProofTypeError for unknown assetLockProof', async () => {
    try {
      await fetchAssetLockTransactionOutput(
        {},
        executionContext,
      );

      expect.fail('should throw UnknownAssetLockProofTypeError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(UnknownAssetLockProofTypeError);
    }
  });

  it('should throw UnknownAssetLockProofTypeError for unknown assetLockProof type', async () => {
    const type = 100;

    const assetLockProofFixture = new ChainAssetLockProof(getChainAssetLockFixture().toObject());
    assetLockProofFixture.getType = () => type;

    try {
      await fetchAssetLockTransactionOutput(
        assetLockProofFixture,
        executionContext,
      );

      expect.fail('should throw UnknownAssetLockProofTypeError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(UnknownAssetLockProofTypeError);
      expect(e.getType()).to.equal(type);
    }
  });
});
