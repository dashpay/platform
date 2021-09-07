const rewiremock = require('rewiremock/node');

const { Transaction } = require('@dashevo/dashcore-lib');

const validateAssetLockTransactionFactory = require('../../../../../lib/identity/stateTransition/assetLockProof/validateAssetLockTransactionFactory');
const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');

const ValidationResult = require('../../../../../lib/validation/ValidationResult');

const { expectValidationError } = require('../../../../../lib/test/expect/expectError');

const InvalidIdentityAssetLockTransactionError = require('../../../../../lib/errors/consensus/basic/identity/InvalidIdentityAssetLockTransactionError');
const IdentityAssetLockTransactionOutputNotFoundError = require('../../../../../lib/errors/consensus/basic/identity/IdentityAssetLockTransactionOutputNotFoundError');
const IdentityAssetLockTransactionOutPointAlreadyExistsError = require('../../../../../lib/errors/consensus/basic/identity/IdentityAssetLockTransactionOutPointAlreadyExistsError');
const InvalidIdentityAssetLockTransactionOutputError = require('../../../../../lib/errors/consensus/basic/identity/InvalidIdentityAssetLockTransactionOutputError');
const InvalidAssetLockTransactionOutputReturnSize = require('../../../../../lib/errors/consensus/basic/identity/InvalidAssetLockTransactionOutputReturnSize');

function getValidateAssetLockTransactionFactoryMock(TransactionClassMock) {
  return rewiremock.proxy(
    '../../../../../lib/identity/stateTransition/assetLockProof/validateAssetLockTransactionFactory',
    {
      '../../../../../node_modules/@dashevo/dashcore-lib': {
        Transaction: TransactionClassMock,
      },
    },
  );
}

describe('validateAssetLockTransactionFactory', () => {
  let stateRepositoryMock;
  let validateAssetLockTransaction;
  let rawTransaction;
  let outputIndex;
  let TransactionClassMock;
  let transactionInstance;

  beforeEach(function beforeEach() {
    rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';
    outputIndex = 0;
    transactionInstance = new Transaction(rawTransaction);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed.returns(false);

    TransactionClassMock = function transactionClassMock() {
      this.outputs = transactionInstance.outputs;
    };

    TransactionClassMock.prototype.getOutPointBuffer = this.sinonSandbox.stub().returns(
      transactionInstance.getOutPointBuffer(outputIndex),
    );

    validateAssetLockTransaction = validateAssetLockTransactionFactory(stateRepositoryMock);
  });

  it('should be valid transaction', async () => {
    rawTransaction = '030000000137feb5676d085133';

    validateAssetLockTransaction = validateAssetLockTransactionFactory(stateRepositoryMock);

    const result = await validateAssetLockTransaction(rawTransaction, outputIndex);

    expectValidationError(result, InvalidIdentityAssetLockTransactionError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1038);
    expect(error.getValidationError()).to.be.instanceOf(Error);

    expect(result.getData()).to.be.undefined();
    expect(stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed).to.not.be.called();
  });

  it('should return IdentityAssetLockTransactionOutputNotFoundError on invalid outputIndex', async () => {
    outputIndex = 42;
    const result = await validateAssetLockTransaction(rawTransaction, outputIndex);

    expectValidationError(result, IdentityAssetLockTransactionOutputNotFoundError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1034);
    expect(error.getOutputIndex()).to.equal(outputIndex);

    expect(result.getData()).to.be.undefined();
    expect(stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed).to.not.be.called();
  });

  it('should point to output with OR_RETURN', async function it() {
    const isDataOut = this.sinonSandbox.stub().returns(false);

    TransactionClassMock = function transactionClassMock() {
      this.outputs = [{
        script: {
          isDataOut,
        },
      }];
    };
    const validateAssetLockTransactionFactoryMocked = getValidateAssetLockTransactionFactoryMock(
      TransactionClassMock,
    );

    validateAssetLockTransaction = validateAssetLockTransactionFactoryMocked(stateRepositoryMock);

    const result = await validateAssetLockTransaction(rawTransaction, outputIndex);

    expectValidationError(result, InvalidIdentityAssetLockTransactionOutputError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1039);
    expect(error.getOutputIndex()).to.equal(outputIndex);
  });

  it('should contain valid public key hash', async function it() {
    const isDataOut = this.sinonSandbox.stub().returns(true);
    const getData = this.sinonSandbox.stub().returns(Buffer.alloc(0));

    TransactionClassMock = function transactionClassMock() {
      this.outputs = [{
        script: {
          isDataOut,
          getData,
        },
      }];
    };
    const validateAssetLockTransactionFactoryMocked = getValidateAssetLockTransactionFactoryMock(
      TransactionClassMock,
    );

    validateAssetLockTransaction = validateAssetLockTransactionFactoryMocked(stateRepositoryMock);

    const result = await validateAssetLockTransaction(rawTransaction, outputIndex);

    expectValidationError(result, InvalidAssetLockTransactionOutputReturnSize);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1037);
    expect(error.getOutputIndex()).to.equal(outputIndex);
  });

  it('should return IdentityAssetLockTransactionOutPointAlreadyExistsError if outPoint was already used', async () => {
    stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed.returns(true);

    const result = await validateAssetLockTransaction(rawTransaction, outputIndex);

    expectValidationError(result, IdentityAssetLockTransactionOutPointAlreadyExistsError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1033);

    const transaction = new Transaction(rawTransaction);

    expect(Buffer.isBuffer(error.getTransactionId())).to.be.true();
    expect(error.getTransactionId()).to.deep.equal(Buffer.from(transaction.id, 'hex'));
    expect(error.getOutputIndex()).to.deep.equal(outputIndex);

    expect(result.getData()).to.be.undefined();
    expect(stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed)
      .to.be.calledOnceWithExactly(
        transaction.getOutPointBuffer(outputIndex),
      );
  });

  it('should return valid result', async () => {
    const result = await validateAssetLockTransaction(rawTransaction, outputIndex);

    expect(result).to.be.an.instanceOf(ValidationResult);

    expect(result.isValid()).to.be.true();

    const initialTransaction = new Transaction(rawTransaction);
    const initialPublicKeyHash = initialTransaction.outputs[outputIndex].script.getData();

    expect(stateRepositoryMock.isAssetLockTransactionOutPointAlreadyUsed)
      .to.be.calledOnceWithExactly(
        initialTransaction.getOutPointBuffer(outputIndex),
      );

    const { transaction, publicKeyHash } = result.getData();
    expect(publicKeyHash).to.deep.equal(initialPublicKeyHash);
    expect(transaction).to.be.an.instanceOf(Transaction);
    expect(transaction.toJSON()).to.deep.equal(initialTransaction.toJSON());
  });
});
