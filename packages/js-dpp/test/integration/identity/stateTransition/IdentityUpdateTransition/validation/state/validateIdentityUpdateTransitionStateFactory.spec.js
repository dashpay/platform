const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');
const validateIdentityUpdateTransitionStateFactory = require('../../../../../../../lib/identity/stateTransition/IdentityUpdateTransition/validation/state/validateIdentityUpdateTransitionStateFactory');
const getIdentityUpdateTransitionFixture = require('../../../../../../../lib/test/fixtures/getIdentityUpdateTransitionFixture');
const IdentityPublicKey = require('../../../../../../../lib/identity/IdentityPublicKey');
const getIdentityFixture = require('../../../../../../../lib/test/fixtures/getIdentityFixture');
const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');
const IdentityNotFoundError = require('../../../../../../../lib/errors/consensus/state/identity/IdentityNotFoundError');
const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError');
const InvalidIdentityRevisionError = require('../../../../../../../lib/errors/consensus/state/identity/InvalidIdentityRevisionError');
const IdentityPublicKeyIsReadOnlyError = require('../../../../../../../lib/errors/consensus/state/identity/IdentityPublicKeyIsReadOnlyError');
const MissedSecurityLevelIdentityPublicKeyError = require('../../../../../../../lib/errors/consensus/state/identity/MissedSecurityLevelIdentityPublicKeyError');

describe('validateIdentityUpdateTransitionStateFactory', () => {
  let validateIdentityUpdateTransitionState;
  let stateRepositoryMock;
  let stateTransition;
  let identity;
  let validatePublicKeysMock;

  beforeEach(async function beforeEach() {
    identity = getIdentityFixture();
    validatePublicKeysMock = this.sinonSandbox.stub()
      .returns(new ValidationResult());

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    validateIdentityUpdateTransitionState = validateIdentityUpdateTransitionStateFactory(
      stateRepositoryMock,
      validatePublicKeysMock,
    );

    stateTransition = getIdentityUpdateTransitionFixture();
    stateTransition.setRevision(identity.getRevision() + 1);
    stateTransition.setDisablePublicKeys(undefined);
    stateTransition.setPublicKeysDisabledAt(undefined);

    const privateKey = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';

    await stateTransition.signByPrivateKey(privateKey, IdentityPublicKey.TYPES.ECDSA_SECP256K1);
  });

  it('should return IdentityNotFoundError', async () => {
    stateRepositoryMock.fetchIdentity.resolves(null);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();
    expect(error).to.be.an.instanceof(IdentityNotFoundError);
    expect(error.getIdentityId()).to.deep.equal(stateTransition.getIdentityId());
  });

  it('should return InvalidIdentityRevisionError', async () => {
    stateTransition.setRevision(identity.getRevision() + 2);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();
    expect(error).to.be.an.instanceof(InvalidIdentityRevisionError);
    expect(error.getIdentityId()).to.deep.equal(stateTransition.getIdentityId());
    expect(error.getCurrentRevision()).to.equal(identity.getRevision());
  });

  it('should return IdentityPublicKeyIsReadOnlyError', async () => {
    identity.getPublicKeyById(0).setReadOnly(true);
    stateTransition.setDisablePublicKeys([0]);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();
    expect(error).to.be.an.instanceof(IdentityPublicKeyIsReadOnlyError);
    expect(error.getPublicKeyIndex()).to.equal(0);
  });

  it('should return MissedSecurityLevelIdentityPublicKeyError', async () => {
    stateTransition.setDisablePublicKeys([0]);
    stateTransition.setPublicKeysDisabledAt(42);
    stateTransition.setAddPublicKeys(undefined);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();
    expect(error).to.be.an.instanceof(MissedSecurityLevelIdentityPublicKeyError);
    expect(error.getSecurityLevel()).to.equal(identity.getPublicKeyById(0).getSecurityLevel());
  });

  it('should pass', async () => {
    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchIdentity)
      .to.be.calledOnceWithExactly(stateTransition.getIdentityId());
    expect(validatePublicKeysMock)
      .to.be.calledOnceWithExactly(identity.getPublicKeys().map((pk) => pk.toObject()));
  });
});
