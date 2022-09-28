const validateStateTransitionKeySignatureFactory = require('../../../../lib/stateTransition/validation/validateStateTransitionKeySignatureFactory');

const getIdentityCreateTransitionFixture = require('../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');
const InvalidStateTransitionSignatureError = require('../../../../lib/errors/consensus/signature/InvalidStateTransitionSignatureError');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../lib/validation/ValidationResult');
const StateTransitionExecutionContext = require('../../../../lib/stateTransition/StateTransitionExecutionContext');

describe('validateStateTransitionKeySignatureFactory', () => {
  let publicKeyHash;
  let stateTransition;
  let stateTransitionHash;
  let verifyHashSignatureMock;
  let validateStateTransitionKeySignature;
  let fetchAssetLockPublicKeyHashMock;
  let executionContext;

  beforeEach(function beforeEach() {
    publicKeyHash = Buffer.alloc(20).fill(1);

    stateTransition = getIdentityCreateTransitionFixture();
    stateTransitionHash = stateTransition.hash({ skipSignature: true });

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    verifyHashSignatureMock = this.sinonSandbox.stub();

    fetchAssetLockPublicKeyHashMock = this.sinonSandbox.stub().resolves(publicKeyHash);

    validateStateTransitionKeySignature = validateStateTransitionKeySignatureFactory(
      verifyHashSignatureMock,
      fetchAssetLockPublicKeyHashMock,
    );
  });

  it('should return invalid result if signature is not valid', async () => {
    verifyHashSignatureMock.returns(false);

    const result = await validateStateTransitionKeySignature(
      stateTransition,
    );

    expectValidationError(result, InvalidStateTransitionSignatureError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(2002);

    expect(fetchAssetLockPublicKeyHashMock).to.be.calledOnceWithExactly(
      stateTransition.getAssetLockProof(),
      executionContext,
    );

    expect(verifyHashSignatureMock).to.be.calledOnceWithExactly(
      stateTransitionHash,
      stateTransition.getSignature(),
      publicKeyHash,
    );
  });

  it('should return valid result if signature is valid', async () => {
    verifyHashSignatureMock.returns(true);

    const result = await validateStateTransitionKeySignature(
      stateTransition,
    );

    expect(result).to.be.instanceof(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(fetchAssetLockPublicKeyHashMock).to.be.calledOnceWithExactly(
      stateTransition.getAssetLockProof(),
      executionContext,
    );

    expect(verifyHashSignatureMock).to.be.calledOnceWithExactly(
      stateTransitionHash,
      stateTransition.getSignature(),
      publicKeyHash,
    );
  });
});
