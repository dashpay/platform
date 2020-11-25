const validateSignatureAgainstAssetLockPublicKeyFactory = require('../../../../lib/identity/stateTransitions/validateSignatureAgainstAssetLockPublicKeyFactory');

const getIdentityCreateTransitionFixture = require('../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');
const InvalidStateTransitionSignatureError = require('../../../../lib/errors/InvalidStateTransitionSignatureError');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

describe('validateSignatureAgainstAssetLockPublicKeyFactory', () => {
  let publicKeyHash;
  let stateTransition;
  let stateTransitionHash;
  let rawStateTransition;
  let createStateTransitionMock;
  let verifyHashSignatureMock;
  let validateSignatureAgainstAssetLockPublicKey;

  beforeEach(function beforeEach() {
    publicKeyHash = Buffer.alloc(20).fill(1);

    stateTransition = getIdentityCreateTransitionFixture();
    stateTransitionHash = stateTransition.hash({ skipSignature: true });
    rawStateTransition = stateTransition.toObject();

    createStateTransitionMock = this.sinonSandbox.stub().resolves(stateTransition);
    verifyHashSignatureMock = this.sinonSandbox.stub();

    validateSignatureAgainstAssetLockPublicKey = validateSignatureAgainstAssetLockPublicKeyFactory(
      createStateTransitionMock,
      verifyHashSignatureMock,
    );
  });

  it('should return invalid result if signature is not valid', async () => {
    verifyHashSignatureMock.returns(false);

    const result = await validateSignatureAgainstAssetLockPublicKey(
      rawStateTransition,
      publicKeyHash,
    );

    expectValidationError(result, InvalidStateTransitionSignatureError);

    expect(createStateTransitionMock).to.be.calledOnceWithExactly(rawStateTransition);
    expect(verifyHashSignatureMock).to.be.calledOnceWithExactly(
      stateTransitionHash,
      stateTransition.getSignature(),
      publicKeyHash,
    );
  });

  it('should return valid result if signature is valid', async () => {
    verifyHashSignatureMock.returns(true);

    const result = await validateSignatureAgainstAssetLockPublicKey(
      rawStateTransition,
      publicKeyHash,
    );

    expect(result).to.be.instanceof(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(createStateTransitionMock).to.be.calledOnceWithExactly(rawStateTransition);
    expect(verifyHashSignatureMock).to.be.calledOnceWithExactly(
      stateTransitionHash,
      stateTransition.getSignature(),
      publicKeyHash,
    );
  });
});
