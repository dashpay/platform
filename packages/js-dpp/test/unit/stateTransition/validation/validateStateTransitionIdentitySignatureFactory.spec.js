const validateStateTransitionSignatureFactory = require('../../../../lib/stateTransition/validation/validateStateTransitionIdentitySignatureFactory');
const ValidationResult = require('../../../../lib/validation/ValidationResult');
const IdentityPublicKey = require('../../../../lib/identity/IdentityPublicKey');
const InvalidIdentityPublicKeyTypeError = require('../../../../lib/errors/consensus/signature/InvalidIdentityPublicKeyTypeError');
const InvalidStateTransitionSignatureError = require('../../../../lib/errors/consensus/signature/InvalidStateTransitionSignatureError');
const MissingPublicKeyError = require('../../../../lib/errors/consensus/signature/MissingPublicKeyError');
const generateRandomIdentifier = require('../../../../lib/test/utils/generateRandomIdentifier');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');
const SomeConsensusError = require('../../../../lib/test/mocks/SomeConsensusError');

describe('validateStateTransitionIdentitySignatureFactory', () => {
  let validateStateTransitionIdentitySignature;
  let stateTransition;
  let ownerId;
  let identity;
  let identityPublicKey;
  let publicKeyId;
  let validateIdentityExistenceResult;
  let validateIdentityExistenceMock;

  beforeEach(function beforeEach() {
    ownerId = generateRandomIdentifier();
    publicKeyId = 1;
    stateTransition = {
      verifySignature: this.sinonSandbox.stub().returns(true),
      getSignaturePublicKeyId: this.sinonSandbox.stub().returns(publicKeyId),
      getSignature: this.sinonSandbox.stub(),
      getOwnerId: this.sinonSandbox.stub().returns(ownerId),
    };

    identityPublicKey = {
      getType: this.sinonSandbox.stub().returns(IdentityPublicKey.TYPES.ECDSA_SECP256K1),
    };

    const getPublicKeyById = this.sinonSandbox.stub().returns(identityPublicKey);

    identity = {
      getPublicKeyById,
    };

    validateIdentityExistenceResult = new ValidationResult();
    validateIdentityExistenceResult.setData(identity);

    validateIdentityExistenceMock = this.sinonSandbox.stub().resolves(
      validateIdentityExistenceResult,
    );

    validateStateTransitionIdentitySignature = validateStateTransitionSignatureFactory(
      validateIdentityExistenceMock,
    );
  });

  it('should pass properly signed state transition', async () => {
    const result = await validateStateTransitionIdentitySignature(
      stateTransition,
    );

    expect(result).to.be.instanceOf(ValidationResult);

    expect(result.isValid()).to.be.true();
    expect(result.getErrors()).to.be.an('array');
    expect(result.getErrors()).to.be.empty();

    expect(validateIdentityExistenceMock).to.be.calledOnceWithExactly(ownerId);
    expect(identity.getPublicKeyById).to.be.calledOnceWithExactly(publicKeyId);
    expect(identityPublicKey.getType).to.be.calledOnce();
    expect(stateTransition.getSignaturePublicKeyId).to.be.calledOnce();
    expect(stateTransition.verifySignature).to.be.calledOnceWithExactly(identityPublicKey);
    expect(stateTransition.getOwnerId).to.be.calledOnceWithExactly();
  });

  it('should return invalid result if owner id doesn\'t exist', async () => {
    const consensusError = new SomeConsensusError('error');

    validateIdentityExistenceResult.addError(consensusError);

    const result = await validateStateTransitionIdentitySignature(
      stateTransition,
    );

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(consensusError);

    expect(validateIdentityExistenceMock).to.be.calledOnceWithExactly(ownerId);
    expect(identity.getPublicKeyById).to.not.be.called();
    expect(identityPublicKey.getType).to.not.be.called();
    expect(stateTransition.getSignaturePublicKeyId).to.not.be.called();
    expect(stateTransition.verifySignature).to.not.be.called();
    expect(stateTransition.getOwnerId).to.be.calledOnceWithExactly();
  });

  it("should return MissingPublicKeyError if the identity doesn't have a matching public key", async () => {
    const type = IdentityPublicKey.TYPES.ECDSA_SECP256K1 + 1;
    identityPublicKey.getType.returns(type);
    identity.getPublicKeyById.returns(undefined);

    const result = await validateStateTransitionIdentitySignature(
      stateTransition,
    );

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.false();
    expect(validateIdentityExistenceMock).to.be.calledOnceWithExactly(ownerId);
    expect(identity.getPublicKeyById).to.be.calledOnceWithExactly(publicKeyId);
    expect(stateTransition.getSignaturePublicKeyId).to.be.calledTwice();
    expect(stateTransition.verifySignature).to.not.be.called();

    expect(result.getErrors()).to.be.an('array');
    expect(result.getErrors()).to.have.lengthOf(1);

    const [error] = result.getErrors();

    expect(error).to.be.instanceOf(MissingPublicKeyError);
    expect(error.getPublicKeyId()).to.equal(publicKeyId);
  });

  it('should return InvalidIdentityPublicKeyTypeError if type is not ECDSA_SECP256K1', async () => {
    const type = IdentityPublicKey.TYPES.ECDSA_SECP256K1 + 1;
    identityPublicKey.getType.returns(type);

    const result = await validateStateTransitionIdentitySignature(
      stateTransition,
    );

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.false();
    expect(validateIdentityExistenceMock).to.be.calledOnceWithExactly(ownerId);
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

    const result = await validateStateTransitionIdentitySignature(
      stateTransition,
    );

    expect(result).to.be.instanceOf(ValidationResult);

    expect(result.isValid()).to.be.false();
    expect(result.getErrors()).to.be.an('array');
    expect(result.getErrors()).to.have.lengthOf(1);

    const [error] = result.getErrors();

    expect(error).to.be.instanceOf(InvalidStateTransitionSignatureError);

    expect(validateIdentityExistenceMock).to.be.calledOnceWithExactly(ownerId);
    expect(identity.getPublicKeyById).to.be.calledOnceWithExactly(publicKeyId);
    expect(identityPublicKey.getType).to.be.calledOnce();
    expect(stateTransition.getSignaturePublicKeyId).to.be.calledOnce();
    expect(stateTransition.verifySignature).to.be.calledOnceWithExactly(identityPublicKey);
  });
});
