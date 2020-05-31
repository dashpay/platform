const { Transaction } = require('@dashevo/dashcore-lib');
const WrongOutPointError = require('@dashevo/dashcore-lib/lib/errors/WrongOutPointError');

const fetchConfirmedAssetLockTransactionOutputFactory = require(
  '../../../lib/stateTransition/fetchConfirmedAssetLockTransactionOutputFactory',
);

const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');

const IdentityAssetLockTransactionNotFoundError = require(
  '../../../lib/errors/IdentityAssetLockTransactionNotFoundError',
);

const IdentityAssetLockTransactionOutputNotFoundError = require(
  '../../../lib/errors/IdentityAssetLockTransactionOutputNotFoundError',
);

const InvalidIdentityAssetLockTransactionOutPointError = require(
  '../../../lib/errors/InvalidIdentityAssetLockTransactionOutPointError',
);

const getRawTransactionFixture = require(
  '../../../lib/test/fixtures/getRawTransactionFixture',
);
const IdentityAssetLockTransactionIsNotConfirmedError = require(
  '../../../lib/errors/IdentityAssetLockTransactionIsNotConfirmedError',
);

describe('fetchConfirmedAssetLockTransactionOutputFactory', () => {
  let rawTransaction;
  let transactionHash;
  let outputIndex;
  let stateRepositoryMock;
  let parseTransactionOutPointBufferMock;
  let fetchConfirmedAssetLockTransactionOutput;
  let lockedOutPoint;
  let skipAssetLockConfirmationValidation;

  beforeEach(function beforeEach() {
    rawTransaction = getRawTransactionFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchTransaction.resolves(rawTransaction);

    lockedOutPoint = 'azW1UgBiB0CmdphN6of4DbT91t0Xv3/c3YUV4CnoV/kAAAAA';

    transactionHash = 'hash';
    outputIndex = 0;

    parseTransactionOutPointBufferMock = this.sinonSandbox.stub().returns({
      transactionHash,
      outputIndex,
    });

    skipAssetLockConfirmationValidation = false;

    fetchConfirmedAssetLockTransactionOutput = fetchConfirmedAssetLockTransactionOutputFactory(
      stateRepositoryMock,
      parseTransactionOutPointBufferMock,
      skipAssetLockConfirmationValidation,
    );
  });

  it('should return lock transaction output', async () => {
    const transaction = new Transaction(rawTransaction.hex);

    const result = await fetchConfirmedAssetLockTransactionOutput(lockedOutPoint);

    expect(result).to.deep.equal(transaction.outputs[outputIndex]);
    expect(parseTransactionOutPointBufferMock).to.be.calledOnceWithExactly(Buffer.from(lockedOutPoint, 'base64'));
    expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(transactionHash);
  });

  it('should throw InvalidIdentityAssetLockTransactionOutPointError if state transition has wrong out point', async () => {
    const wrongOutPointError = new WrongOutPointError('Outpoint is wrong');

    parseTransactionOutPointBufferMock.throws(wrongOutPointError);

    try {
      await fetchConfirmedAssetLockTransactionOutput(lockedOutPoint);

      expect.fail('should throw InvalidIdentityAssetLockTransactionOutPointError');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidIdentityAssetLockTransactionOutPointError);
      expect(e.message).to.equal(`Invalid Identity out point: ${wrongOutPointError.message}`);
      expect(parseTransactionOutPointBufferMock).to.be.calledOnceWithExactly(Buffer.from(lockedOutPoint, 'base64'));
      expect(stateRepositoryMock.fetchTransaction).to.be.not.called();
    }
  });

  it('should throw IdentityAssetLockTransactionNotFoundError if lock transaction is not found', async () => {
    stateRepositoryMock.fetchTransaction.resolves(null);

    try {
      await fetchConfirmedAssetLockTransactionOutput(lockedOutPoint);

      expect.fail('should throw IdentityAssetLockTransactionNotFoundError');
    } catch (e) {
      expect(e).to.be.an.instanceof(IdentityAssetLockTransactionNotFoundError);
      expect(e.getTransactionHash()).to.deep.equal(transactionHash);
      expect(parseTransactionOutPointBufferMock).to.be.calledOnceWithExactly(Buffer.from(lockedOutPoint, 'base64'));
      expect(stateRepositoryMock.fetchTransaction).to.calledOnceWithExactly(transactionHash);
    }
  });

  it('should throw IdentityAssetLockTransactionOutputNotFoundError if transaction has no output with given index', async () => {
    outputIndex = 10;

    parseTransactionOutPointBufferMock.returns({
      transactionHash,
      outputIndex,
    });

    try {
      await fetchConfirmedAssetLockTransactionOutput(lockedOutPoint);

      expect.fail('should throw IdentityAssetLockTransactionOutputNotFoundError');
    } catch (e) {
      expect(e).to.be.an.instanceof(IdentityAssetLockTransactionOutputNotFoundError);
      expect(e.getOutputIndex()).to.equal(outputIndex);
      expect(parseTransactionOutPointBufferMock).to.be.calledOnceWithExactly(Buffer.from(lockedOutPoint, 'base64'));
      expect(stateRepositoryMock.fetchTransaction).to.calledOnceWithExactly(transactionHash);
    }
  });

  it('should throw IdentityAssetLockTransactionIsNotConfirmedError if transaction is not chainlocked and not instantlocked', async () => {
    rawTransaction.chainlock = false;
    rawTransaction.instantlock = false;

    try {
      await fetchConfirmedAssetLockTransactionOutput(lockedOutPoint);

      expect.fail('should throw IdentityAssetLockTransactionIsNotConfirmedError');
    } catch (e) {
      expect(e).to.be.an.instanceof(IdentityAssetLockTransactionIsNotConfirmedError);
      expect(e.getTransactionHash()).to.deep.equal(transactionHash);
    }
  });

  it('should return lock transaction output if transaction is only chainlocked', async () => {
    rawTransaction.instantlock = false;

    const transaction = new Transaction(rawTransaction.hex);

    const result = await fetchConfirmedAssetLockTransactionOutput(lockedOutPoint);

    expect(result).to.deep.equal(transaction.outputs[outputIndex]);
    expect(parseTransactionOutPointBufferMock).to.be.calledOnceWithExactly(Buffer.from(lockedOutPoint, 'base64'));
    expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(transactionHash);
  });

  it('should return lock transaction output if transaction is only instantlocked', async () => {
    rawTransaction.chainlock = false;

    const transaction = new Transaction(rawTransaction.hex);

    const result = await fetchConfirmedAssetLockTransactionOutput(lockedOutPoint);

    expect(result).to.deep.equal(transaction.outputs[outputIndex]);
    expect(parseTransactionOutPointBufferMock).to.be.calledOnceWithExactly(Buffer.from(lockedOutPoint, 'base64'));
    expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(transactionHash);
  });

  it('should return unconfirmed lock transaction output if skipAssetLockConfirmationValidation is true', async () => {
    rawTransaction.chainlock = false;
    rawTransaction.instantlock = false;

    skipAssetLockConfirmationValidation = true;

    fetchConfirmedAssetLockTransactionOutput = fetchConfirmedAssetLockTransactionOutputFactory(
      stateRepositoryMock,
      parseTransactionOutPointBufferMock,
      skipAssetLockConfirmationValidation,
    );

    const transaction = new Transaction(rawTransaction.hex);

    const result = await fetchConfirmedAssetLockTransactionOutput(lockedOutPoint);

    expect(result).to.deep.equal(transaction.outputs[outputIndex]);
    expect(parseTransactionOutPointBufferMock).to.be.calledOnceWithExactly(Buffer.from(lockedOutPoint, 'base64'));
    expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(transactionHash);
  });
});
