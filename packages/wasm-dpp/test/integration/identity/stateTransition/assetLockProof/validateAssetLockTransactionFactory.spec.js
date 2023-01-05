const { Transaction, PrivateKey, Script } = require('@dashevo/dashcore-lib');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const AbstractConsensusError = require('@dashevo/dpp/lib/errors/consensus/AbstractConsensusError');
const { expect } = require('chai');
const { default: loadWasmDpp } = require('../../../../../dist');

describe('validateAssetLockTransactionFactory', () => {
  let stateRepositoryMock;
  let rawTransaction;
  let outputIndex;
  let transactionMock;
  let executionContext;
  let expectValidationError;

  let StateTransitionExecutionContext;
  let InvalidIdentityAssetLockTransactionError;
  let IdentityAssetLockTransactionOutputNotFoundError;
  let InvalidIdentityAssetLockTransactionOutputError;
  let InvalidAssetLockTransactionOutputReturnSizeError;
  let IdentityAssetLockTransactionOutPointAlreadyExistsError;
  let ValidationResult;

  let validateAssetLockTransaction;
  let validateAssetLockTransactionDPP;

  before(async () => {
    ({
      StateTransitionExecutionContext,
      ValidationResult,
      InvalidIdentityAssetLockTransactionError,
      IdentityAssetLockTransactionOutputNotFoundError,
      InvalidIdentityAssetLockTransactionOutputError,
      InvalidAssetLockTransactionOutputReturnSizeError,
      IdentityAssetLockTransactionOutPointAlreadyExistsError,
      validateAssetLockTransaction: validateAssetLockTransactionDPP,
    } = await loadWasmDpp());

    expectValidationError = (
      result,
      errorClass = AbstractConsensusError,
      count = 1,
    ) => {
      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.getErrors()).to.have.lengthOf(count);

      result.getErrors().forEach((error) => expect(error).to.be.an.instanceOf(errorClass));
    };
  });

  beforeEach(function beforeEach() {
    rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';
    outputIndex = 0;

    executionContext = new StateTransitionExecutionContext();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed.returns(false);

    validateAssetLockTransaction = (tx, index, context) => validateAssetLockTransactionDPP(
      stateRepositoryMock,
      tx,
      index,
      context,
    );
  });

  afterEach(() => {
    if (transactionMock) {
      transactionMock.restore();
    }
  });

  it('should be valid transaction', async () => {
    rawTransaction = '030000000137feb5676d085133';

    const result = await validateAssetLockTransaction(
      rawTransaction,
      outputIndex,
      executionContext,
    );

    expectValidationError(result, InvalidIdentityAssetLockTransactionError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1038);
    expect(error.getValidationError()).to.exist();

    expect(result.getData()).to.be.undefined();
    expect(stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed).to.not.be.called();
  });

  it('should return IdentityAssetLockTransactionOutputNotFoundError on invalid outputIndex', async () => {
    outputIndex = 42;

    const result = await validateAssetLockTransaction(
      rawTransaction,
      outputIndex,
      executionContext,
    );

    expectValidationError(result, IdentityAssetLockTransactionOutputNotFoundError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1034);
    expect(error.getOutputIndex()).to.equal(outputIndex);

    expect(result.getData()).to.be.undefined();
    expect(stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed).to.not.be.called();
  });

  it('should point to output with OR_RETURN', async () => {
    const regularTransaction = new Transaction(Buffer.from(rawTransaction, 'hex'));
    regularTransaction.outputs[0]
      .setScript(Script.buildPublicKeyHashOut(new PrivateKey().toAddress()).toString());

    const result = await validateAssetLockTransaction(
      regularTransaction.toBuffer().toString('hex'),
      outputIndex,
      executionContext,
    );

    expectValidationError(result, InvalidIdentityAssetLockTransactionOutputError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1039);
    expect(error.getOutputIndex()).to.equal(outputIndex);
  });

  it('should contain valid public key hash', async () => {
    const regularTransaction = new Transaction(Buffer.from(rawTransaction, 'hex'));
    regularTransaction.outputs[0]
      .setScript(Script.buildDataOut(Buffer.alloc(0)));

    const result = await validateAssetLockTransaction(
      regularTransaction.toBuffer().toString('hex'),
      outputIndex,
      executionContext,
    );

    expectValidationError(result, InvalidAssetLockTransactionOutputReturnSizeError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1037);
    expect(error.getOutputIndex()).to.equal(outputIndex);
  });

  it('should return IdentityAssetLockTransactionOutPointAlreadyExistsError if outPoint was already used', async () => {
    stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed.returns(true);

    const result = await validateAssetLockTransaction(
      rawTransaction,
      outputIndex,
      executionContext,
    );

    expectValidationError(result, IdentityAssetLockTransactionOutPointAlreadyExistsError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1033);

    const transaction = new Transaction(rawTransaction);

    expect(error.getTransactionId()).to.deep.equal(Buffer.from(transaction.id, 'hex'));
    expect(error.getOutputIndex()).to.deep.equal(outputIndex);

    expect(result.getData()).to.be.undefined();

    const { args } = stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed.firstCall;
    expect(args[0]).to.deep.equal(transaction.getOutPointBuffer(outputIndex));
    expect(args[1]).to.be.instanceOf(StateTransitionExecutionContext);
  });

  it('should return valid result', async () => {
    const result = await validateAssetLockTransaction(
      rawTransaction,
      outputIndex,
      executionContext,
    );

    expect(result).to.be.an.instanceOf(ValidationResult);

    expect(result.isValid()).to.be.true();

    const initialTransaction = new Transaction(rawTransaction);
    const initialPublicKeyHash = initialTransaction.outputs[outputIndex].script.getData();

    const { args } = stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed.firstCall;
    expect(args[0]).to.deep.equal(initialTransaction.getOutPointBuffer(outputIndex));
    expect(args[1]).to.be.instanceOf(StateTransitionExecutionContext);

    const { transaction, publicKeyHash } = result.getData();
    expect(publicKeyHash).to.deep.equal(initialPublicKeyHash);
    const parsedTx = new Transaction(Buffer.from(transaction));
    expect(parsedTx.toJSON()).to.deep.equal(initialTransaction.toJSON());
  });
});
