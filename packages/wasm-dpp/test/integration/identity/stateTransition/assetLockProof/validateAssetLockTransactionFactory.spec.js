const { Transaction, PrivateKey, Script } = require('@dashevo/dashcore-lib');
const { expect } = require('chai');

const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');
const { default: loadWasmDpp } = require('../../../../../dist');
const { expectValidationError } = require('../../../../../lib/test/expect/expectError');

describe.skip('validateAssetLockTransactionFactory', () => {
  let stateRepositoryMock;
  let rawTransaction;
  let outputIndex;
  let executionContext;

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
  });

  beforeEach(function beforeEach() {
    rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';
    outputIndex = 0;

    executionContext = new StateTransitionExecutionContext();

    stateRepositoryMock = createStateRepositoryMock(this.sinon);

    stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed.resolves(false);

    validateAssetLockTransaction = (tx, index, context) => validateAssetLockTransactionDPP(
      stateRepositoryMock,
      tx,
      index,
      context,
    );
  });

  it('should be valid transaction', async () => {
    rawTransaction = '030000000137feb5676d085133';

    const result = await validateAssetLockTransaction(
      rawTransaction,
      outputIndex,
      executionContext,
    );

    await expectValidationError(result, InvalidIdentityAssetLockTransactionError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1038);
    expect(error.getErrorMessage()).to.not.be.empty();

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

    await expectValidationError(result, IdentityAssetLockTransactionOutputNotFoundError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1034);
    expect(error.getOutputIndex()).to.equal(outputIndex);

    expect(result.getData()).to.be.undefined();
    expect(stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed).to.not.be.called();
  });

  it('should point to output with OR_RETURN', async () => {
    const regularTransaction = new Transaction(Buffer.from(rawTransaction, 'hex'));
    // Mess up expected TX output script
    regularTransaction.outputs[0]
      .setScript(Script.buildPublicKeyHashOut(new PrivateKey().toAddress()).toString());

    const result = await validateAssetLockTransaction(
      regularTransaction.toBuffer().toString('hex'),
      outputIndex,
      executionContext,
    );

    await expectValidationError(result, InvalidIdentityAssetLockTransactionOutputError);

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

    await expectValidationError(result, InvalidAssetLockTransactionOutputReturnSizeError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1037);
    expect(error.getOutputIndex()).to.equal(outputIndex);
  });

  it('should return IdentityAssetLockTransactionOutPointAlreadyExistsError if outPoint was'
    + ' already used', async function shouldReturn() {
    stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed.resolves(true);

    const result = await validateAssetLockTransaction(
      rawTransaction,
      outputIndex,
      executionContext,
    );

    await expectValidationError(result, IdentityAssetLockTransactionOutPointAlreadyExistsError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1033);

    const transaction = new Transaction(rawTransaction);

    expect(error.getTransactionId()).to.deep.equal(Buffer.from(transaction.id, 'hex'));
    expect(error.getOutputIndex()).to.deep.equal(outputIndex);

    expect(result.getData()).to.be.undefined();
    expect(stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed)
      .to.be.calledOnceWithExactly(
        this.sinon.match((val) => Buffer.from(val)
          .equals(transaction.getOutPointBuffer(outputIndex))),
        this.sinon.match.instanceOf(StateTransitionExecutionContext),
      );
  });

  it('should return valid result', async function shouldReturn() {
    const result = await validateAssetLockTransaction(
      rawTransaction,
      outputIndex,
      executionContext,
    );

    expect(result).to.be.an.instanceOf(ValidationResult);

    expect(result.isValid()).to.be.true();

    const initialTransaction = new Transaction(rawTransaction);
    const initialPublicKeyHash = initialTransaction.outputs[outputIndex].script.getData();

    expect(stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed)
      .to.be.calledOnceWithExactly(
        this.sinon.match((val) => Buffer.from(val)
          .equals(initialTransaction.getOutPointBuffer(outputIndex))),
        this.sinon.match.instanceOf(StateTransitionExecutionContext),
      );

    const { transaction, publicKeyHash } = result.getData();
    expect(publicKeyHash).to.deep.equal(initialPublicKeyHash);
    expect(new Transaction(Buffer.from(transaction)).toJSON())
      .to.deep.equal(initialTransaction.toJSON());
  });
});
