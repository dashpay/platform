const validateStateTransitionSignatureFactory = require('../../../../lib/stateTransition/validation/validateStateTransitionSignatureFactory');
const createDataProviderMock = require('../../../../lib/test/mocks/createDataProviderMock');
const ValidationResult = require('../../../../lib/validation/ValidationResult');
const IdentityPublicKey = require('../../../../lib/identity/IdentityPublicKey');
const InvalidIdentityPublicKeyTypeError = require('../../../../lib/errors/InvalidIdentityPublicKeyTypeError');
const InvalidStateTransitionSignatureError = require('../../../../lib/errors/InvalidStateTransitionSignatureError');

describe('validateStateTransitionSignatureFactory', () => {
  let validateStateTransitionSignature;
  let dataProviderMock;
  let stateTransition;
  let userId;
  let identity;
  let identityPublicKey;
  let publicKeyId;

  beforeEach(function beforeEach() {
    publicKeyId = 1;
    stateTransition = {
      verifySignature: this.sinonSandbox.stub().returns(true),
      getSignaturePublicKeyId: this.sinonSandbox.stub().returns(publicKeyId),
      getSignature: this.sinonSandbox.stub(),
    };

    identityPublicKey = {
      getType: this.sinonSandbox.stub().returns(IdentityPublicKey.TYPES.ECDSA_SECP256K1),
    };

    const getPublicKeyById = this.sinonSandbox.stub().returns(identityPublicKey);

    identity = {
      getPublicKeyById,
    };

    dataProviderMock = createDataProviderMock(this.sinonSandbox);
    dataProviderMock.fetchIdentity.resolves(identity);

    validateStateTransitionSignature = validateStateTransitionSignatureFactory(
      dataProviderMock,
    );
  });

  it('should pass properly signed state transition', async () => {
    const result = await validateStateTransitionSignature(
      stateTransition,
      userId,
    );

    expect(result).to.be.instanceOf(ValidationResult);

    expect(result.isValid()).to.be.true();
    expect(result.getErrors()).to.be.an('array');
    expect(result.getErrors()).to.be.empty();

    expect(dataProviderMock.fetchIdentity).to.be.calledOnceWithExactly(userId);
    expect(identity.getPublicKeyById).to.be.calledOnceWithExactly(publicKeyId);
    expect(identityPublicKey.getType).to.be.calledOnce();
    expect(stateTransition.getSignaturePublicKeyId).to.be.calledOnce();
    expect(stateTransition.verifySignature).to.be.calledOnceWithExactly(identityPublicKey);
  });

  it('should return InvalidIdentityPublicKeyTypeError if type is not ECDSA_SECP256K1', async () => {
    const type = IdentityPublicKey.TYPES.ECDSA_SECP256K1 + 1;
    identityPublicKey.getType.returns(type);

    const result = await validateStateTransitionSignature(
      stateTransition,
      userId,
    );

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.false();
    expect(dataProviderMock.fetchIdentity).to.be.calledOnceWithExactly(userId);
    expect(identity.getPublicKeyById).to.be.calledOnceWithExactly(publicKeyId);
    expect(identityPublicKey.getType).to.be.calledTwice();
    expect(stateTransition.getSignaturePublicKeyId).to.be.calledOnce();
    expect(stateTransition.verifySignature).to.not.be.called();

    expect(result.getErrors()).to.be.an('array');
    expect(result.getErrors()).to.have.lengthOf(1);

    const [error] = result.getErrors();

    expect(error).to.be.instanceOf(InvalidIdentityPublicKeyTypeError);
    expect(error.getType()).to.equal(type);
  });

  it('should return InvalidStateTransitionSignatureError if signature is invalid', async () => {
    stateTransition.verifySignature.returns(false);

    const result = await validateStateTransitionSignature(
      stateTransition,
      userId,
    );

    expect(result).to.be.instanceOf(ValidationResult);

    expect(result.isValid()).to.be.false();
    expect(result.getErrors()).to.be.an('array');
    expect(result.getErrors()).to.have.lengthOf(1);

    const [error] = result.getErrors();

    expect(error).to.be.instanceOf(InvalidStateTransitionSignatureError);
    expect(error.getRawStateTransition()).to.equal(stateTransition);

    expect(dataProviderMock.fetchIdentity).to.be.calledOnceWithExactly(userId);
    expect(identity.getPublicKeyById).to.be.calledOnceWithExactly(publicKeyId);
    expect(identityPublicKey.getType).to.be.calledOnce();
    expect(stateTransition.getSignaturePublicKeyId).to.be.calledOnce();
    expect(stateTransition.verifySignature).to.be.calledOnceWithExactly(identityPublicKey);
  });
});
