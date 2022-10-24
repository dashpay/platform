const validateStateTransitionKeySignatureFactory = require('../../../../lib/stateTransition/validation/validateStateTransitionKeySignatureFactory');

const getIdentityCreateTransitionFixture = require('../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');
const getIdentityTopUpTransitionFixture = require('../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');
const InvalidStateTransitionSignatureError = require('../../../../lib/errors/consensus/signature/InvalidStateTransitionSignatureError');
const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../lib/validation/ValidationResult');
const StateTransitionExecutionContext = require('../../../../lib/stateTransition/StateTransitionExecutionContext');
const IdentityNotFoundError = require('../../../../lib/errors/consensus/signature/IdentityNotFoundError');

describe('validateStateTransitionKeySignatureFactory', () => {
  let publicKeyHash;
  let stateTransition;
  let stateTransitionHash;
  let verifyHashSignatureMock;
  let validateStateTransitionKeySignature;
  let fetchAssetLockPublicKeyHashMock;
  let executionContext;
  let stateRepositoryMock;

  beforeEach(function beforeEach() {
    publicKeyHash = Buffer.alloc(20).fill(1);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateTransition = getIdentityCreateTransitionFixture();
    stateTransitionHash = stateTransition.hash({ skipSignature: true });

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    verifyHashSignatureMock = this.sinonSandbox.stub();

    fetchAssetLockPublicKeyHashMock = this.sinonSandbox.stub().resolves(publicKeyHash);

    validateStateTransitionKeySignature = validateStateTransitionKeySignatureFactory(
      verifyHashSignatureMock,
      fetchAssetLockPublicKeyHashMock,
      stateRepositoryMock,
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

  it('should return IdentityNotFoundError if identity not exist on topup transaction', async () => {
    stateTransition = getIdentityTopUpTransitionFixture();
    stateRepositoryMock.fetchIdentity.resolves(null);

    const result = await validateStateTransitionKeySignature(
      stateTransition,
    );

    expectValidationError(result, IdentityNotFoundError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(2000);

    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(
      stateTransition.getIdentityId(),
      new StateTransitionExecutionContext(),
    );
  });
});
