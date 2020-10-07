const validateAssetLockTransactionFactory = require('../../../../../lib/identity/stateTransitions/identityCreateTransition/validateAssetLockTransactionFactory');
const IdentityCreateTransition = require('../../../../../lib/identity/stateTransitions/identityCreateTransition/IdentityCreateTransition');
const stateTransitionTypes = require('../../../../../lib/stateTransition/stateTransitionTypes');

const InvalidIdentityAssetLockTransactionOutputError = require(
  '../../../../../lib/errors/InvalidIdentityAssetLockTransactionOutputError',
);
const InvalidStateTransitionSignatureError = require(
  '../../../../../lib/errors/InvalidStateTransitionSignatureError',
);
const IdentityAssetLockTransactionNotFoundError = require(
  '../../../../../lib/errors/IdentityAssetLockTransactionNotFoundError',
);
const { expectValidationError } = require(
  '../../../../../lib/test/expect/expectError',
);

describe('validateAssetLockTransactionFactory', () => {
  let validateAssetLockTransaction;
  let stateTransition;
  let privateKey;
  let fetchConfirmedAssetLockTransactionOutputMock;
  let output;

  beforeEach(function beforeEach() {
    privateKey = 'af432c476f65211f45f48f1d42c9c0b497e56696aa1736b40544ef1a496af837';

    stateTransition = new IdentityCreateTransition({
      protocolVersion: 0,
      type: stateTransitionTypes.IDENTITY_CREATE,
      lockedOutPoint: 'azW1UgBiB0CmdphN6of4DbT91t0Xv3/c3YUV4CnoV/kAAAAA',
      publicKeys: [
        {
          id: 0,
          type: 1,
          data: 'Alw8x/v8UvcQyUFJf9AYdsGJFx6iJ0WPUBr8s4opfWW0',
        },
      ],
    });
    stateTransition.signByPrivateKey(privateKey);

    const script = {
      isDataOut: this.sinonSandbox.stub()
        .returns(true),
      getData: this.sinonSandbox.stub()
        .returns(Buffer.from('152073ca2300a86b510fa2f123d3ea7da3af68dc', 'hex')),
    };

    output = {
      script,
    };

    fetchConfirmedAssetLockTransactionOutputMock = this.sinonSandbox.stub().resolves(output);

    validateAssetLockTransaction = validateAssetLockTransactionFactory(
      fetchConfirmedAssetLockTransactionOutputMock,
    );
  });

  it('should return valid result', async () => {
    const result = await validateAssetLockTransaction(stateTransition);

    expect(result.isValid()).to.be.true();

    expect(fetchConfirmedAssetLockTransactionOutputMock).to.be.calledOnceWithExactly(
      stateTransition.getLockedOutPoint().toString(),
    );
  });

  it('should check transaction output is a valid OP_RETURN output', async () => {
    output.script.isDataOut.returns(false);

    const result = await validateAssetLockTransaction(stateTransition);

    expectValidationError(result, InvalidIdentityAssetLockTransactionOutputError);

    const [error] = result.getErrors();

    expect(error.message).to.equal('Invalid identity lock transaction output: Output is not a valid standard OP_RETURN output');
    expect(error.getOutput()).to.deep.equal(output);
  });

  it('should return invalid result if transaction output script data has size < 20', async () => {
    output.script.getData.returns(Buffer.from('1'.repeat(19)));

    const result = await validateAssetLockTransaction(stateTransition);

    expectValidationError(result, InvalidIdentityAssetLockTransactionOutputError);

    const [error] = result.getErrors();

    expect(error.message).to.equal('Invalid identity lock transaction output: Output has invalid public key hash');
    expect(error.getOutput()).to.deep.equal(output);
  });

  it('should return invalid result if transaction output script data has size > 20', async () => {
    output.script.getData.returns(Buffer.from('1'.repeat(21)));

    const result = await validateAssetLockTransaction(stateTransition);

    expectValidationError(result, InvalidIdentityAssetLockTransactionOutputError);

    const [error] = result.getErrors();

    expect(error.message).to.equal('Invalid identity lock transaction output: Output has invalid public key hash');
    expect(error.getOutput()).to.deep.equal(output);
  });

  it('should return invalid result if state transition has wrong signature', async () => {
    stateTransition.signByPrivateKey('17bc80e9cc3d9082925502342acd2e308ab391c45f753f619b05029b4a487d8f');

    const result = await validateAssetLockTransaction(stateTransition);

    expectValidationError(result, InvalidStateTransitionSignatureError);

    const [error] = result.getErrors();

    expect(error.getRawStateTransition()).to.deep.equal(stateTransition);
  });

  it('should return an invalid result if transaction is not found', async () => {
    const transactionHash = 'f1c1cbc37b5d5543eeb126a53de7863ea2b9d5dbd03b981337bbda76cc6d771c';
    const notFoundError = new IdentityAssetLockTransactionNotFoundError(transactionHash);

    fetchConfirmedAssetLockTransactionOutputMock.throws(notFoundError);

    const result = await validateAssetLockTransaction(stateTransition);

    expectValidationError(result, IdentityAssetLockTransactionNotFoundError);

    const [error] = result.getErrors();

    expect(error.getTransactionHash()).to.equal(transactionHash);
  });
});
