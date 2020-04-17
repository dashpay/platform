const { Transaction } = require('@dashevo/dashcore-lib');
const WrongOutPointError = require('@dashevo/dashcore-lib/lib/errors/WrongOutPointError');

const getLockedTransactionOutputFactory = require('../../../lib/stateTransition/getLockedTransactionOutputFactory');

const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');

const IdentityLockTransactionNotFoundError = require(
  '../../../lib/errors/IdentityLockTransactionNotFoundError',
);
const InvalidIdentityOutPointError = require(
  '../../../lib/errors/InvalidIdentityOutPointError',
);

describe('getLockedTransactionOutputFactory', () => {
  let rawTransaction;
  let transactionHash;
  let outputIndex;
  let stateRepositoryMock;
  let parseTransactionOutPointBufferMock;
  let getLockedTransactionOutput;
  let lockedOutPoint;

  beforeEach(function beforeEach() {
    rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchTransaction.resolves(rawTransaction);

    lockedOutPoint = 'azW1UgBiB0CmdphN6of4DbT91t0Xv3/c3YUV4CnoV/kAAAAA';

    transactionHash = 'hash';
    outputIndex = 0;

    parseTransactionOutPointBufferMock = this.sinonSandbox.stub().returns({
      transactionHash,
      outputIndex,
    });

    getLockedTransactionOutput = getLockedTransactionOutputFactory(
      stateRepositoryMock,
      parseTransactionOutPointBufferMock,
    );
  });

  it('should return lock transaction output', async () => {
    const transaction = new Transaction(rawTransaction);

    const result = await getLockedTransactionOutput(lockedOutPoint);

    expect(result).to.deep.equal(transaction.outputs[outputIndex]);
    expect(parseTransactionOutPointBufferMock).to.be.calledOnceWithExactly(Buffer.from(lockedOutPoint, 'base64'));
    expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(transactionHash);
  });

  it('should throw InvalidIdentityOutPointError if state transition has wrong out point', async () => {
    const wrongOutPointError = new WrongOutPointError('Outpoint is wrong');

    parseTransactionOutPointBufferMock.throws(wrongOutPointError);

    try {
      await getLockedTransactionOutput(lockedOutPoint);

      expect.fail('should throw InvalidIdentityOutPointError');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidIdentityOutPointError);
      expect(e.message).to.equal(`Invalid Identity out point: ${wrongOutPointError.message}`);
      expect(parseTransactionOutPointBufferMock).to.be.calledOnceWithExactly(Buffer.from(lockedOutPoint, 'base64'));
      expect(stateRepositoryMock.fetchTransaction).to.be.not.called();
    }
  });

  it('should throw IdentityLockTransactionNotFoundError if lock transaction is not found', async () => {
    stateRepositoryMock.fetchTransaction.resolves(null);

    try {
      await getLockedTransactionOutput(lockedOutPoint);

      expect.fail('should throw InvalidIdentityOutPointError');
    } catch (e) {
      expect(e).to.be.an.instanceof(IdentityLockTransactionNotFoundError);
      expect(e.getTransactionHash()).to.deep.equal(transactionHash);
      expect(parseTransactionOutPointBufferMock).to.be.calledOnceWithExactly(Buffer.from(lockedOutPoint, 'base64'));
      expect(stateRepositoryMock.fetchTransaction).to.calledOnceWithExactly(transactionHash);
    }
  });

  it('should throw InvalidIdentityOutPointError if transaction has no output with given index', async () => {
    outputIndex = 10;

    parseTransactionOutPointBufferMock.returns({
      transactionHash,
      outputIndex,
    });

    try {
      await getLockedTransactionOutput(lockedOutPoint);

      expect.fail('should throw InvalidIdentityOutPointError');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidIdentityOutPointError);
      expect(e.message).to.equal(`Invalid Identity out point: Output with index ${outputIndex} not found`);
      expect(parseTransactionOutPointBufferMock).to.be.calledOnceWithExactly(Buffer.from(lockedOutPoint, 'base64'));
      expect(stateRepositoryMock.fetchTransaction).to.calledOnceWithExactly(transactionHash);
    }
  });
});
