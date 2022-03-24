const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');
const validateIdentityUpdateTransitionStateFactory = require('../../../../../../../lib/identity/stateTransition/IdentityUpdateTransition/validation/state/validateIdentityUpdateTransitionStateFactory');
const getIdentityUpdateTransitionFixture = require('../../../../../../../lib/test/fixtures/getIdentityUpdateTransitionFixture');
const IdentityPublicKey = require('../../../../../../../lib/identity/IdentityPublicKey');
const getIdentityFixture = require('../../../../../../../lib/test/fixtures/getIdentityFixture');
const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');
const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError');
const InvalidIdentityRevisionError = require('../../../../../../../lib/errors/consensus/state/identity/InvalidIdentityRevisionError');
const IdentityPublicKeyIsReadOnlyError = require('../../../../../../../lib/errors/consensus/state/identity/IdentityPublicKeyIsReadOnlyError');
const MissedSecurityLevelIdentityPublicKeyError = require('../../../../../../../lib/errors/consensus/state/identity/MissedSecurityLevelIdentityPublicKeyError');
const IdentityPublicKeyDisabledAtWindowViolationError = require('../../../../../../../lib/errors/consensus/state/identity/IdentityPublicKeyDisabledAtWindowViolationError');
const InvalidIdentityPublicKeyIdError = require('../../../../../../../lib/errors/consensus/state/identity/InvalidIdentityPublicKeyIdError');
const SomeConsensusError = require('../../../../../../../lib/test/mocks/SomeConsensusError');

describe('validateIdentityUpdateTransitionStateFactory', () => {
  let validateIdentityUpdateTransitionState;
  let stateRepositoryMock;
  let stateTransition;
  let identity;
  let validatePublicKeysMock;
  let validatePublicKeysAreEnabledMock;
  let fakeTime;
  let blockTime;

  beforeEach(async function beforeEach() {
    identity = getIdentityFixture();
    validatePublicKeysMock = this.sinonSandbox.stub()
      .returns(new ValidationResult());
    validatePublicKeysAreEnabledMock = this.sinonSandbox.stub()
      .returns(new ValidationResult());

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    blockTime = new Date().getTime() / 1000;

    const abciHeader = {
      time: {
        seconds: blockTime,
      },
    };

    stateRepositoryMock.fetchLatestPlatformBlockHeader.resolves(abciHeader);

    validateIdentityUpdateTransitionState = validateIdentityUpdateTransitionStateFactory(
      stateRepositoryMock,
      validatePublicKeysMock,
      validatePublicKeysAreEnabledMock,
    );

    stateTransition = getIdentityUpdateTransitionFixture();
    stateTransition.setRevision(identity.getRevision() + 1);
    stateTransition.setPublicKeyIdsToDisable(undefined);
    stateTransition.setPublicKeysDisabledAt(undefined);

    const privateKey = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';

    await stateTransition.signByPrivateKey(privateKey, IdentityPublicKey.TYPES.ECDSA_SECP256K1);

    fakeTime = this.sinonSandbox.useFakeTimers(new Date());
  });

  afterEach(() => {
    fakeTime.reset();
  });

  it('should return InvalidIdentityRevisionError', async () => {
    stateTransition.setRevision(identity.getRevision() + 2);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expectValidationError(result, InvalidIdentityRevisionError);

    const [error] = result.getErrors();
    expect(error.getIdentityId()).to.deep.equal(stateTransition.getIdentityId());
    expect(error.getCurrentRevision()).to.equal(identity.getRevision());
  });

  it('should return IdentityPublicKeyIsReadOnlyError', async () => {
    identity.getPublicKeyById(0).setReadOnly(true);
    stateTransition.setPublicKeyIdsToDisable([0]);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expectValidationError(result, IdentityPublicKeyIsReadOnlyError);

    const [error] = result.getErrors();
    expect(error.getPublicKeyIndex()).to.equal(0);
  });

  it('should return MissedSecurityLevelIdentityPublicKeyError', async () => {
    stateTransition.setPublicKeyIdsToDisable([0]);
    stateTransition.setPublicKeysDisabledAt(new Date());
    stateTransition.setPublicKeysToAdd(undefined);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expectValidationError(result, MissedSecurityLevelIdentityPublicKeyError);

    const [error] = result.getErrors();
    expect(error.getSecurityLevel()).to.equal(identity.getPublicKeyById(0).getSecurityLevel());
  });

  it('should return invalid result if disabledAt have violated time window', async () => {
    stateTransition.setPublicKeyIdsToDisable([1]);
    stateTransition.setPublicKeysDisabledAt(new Date());

    const timeWindowStart = new Date(blockTime * 1000);
    timeWindowStart.setMinutes(
      timeWindowStart.getMinutes() - 5,
    );

    const timeWindowEnd = new Date(blockTime * 1000);
    timeWindowEnd.setMinutes(
      timeWindowEnd.getMinutes() + 5,
    );

    stateTransition.publicKeysDisabledAt.setMinutes(
      stateTransition.publicKeysDisabledAt.getMinutes() - 6,
    );

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expectValidationError(result, IdentityPublicKeyDisabledAtWindowViolationError);

    const [error] = result.getErrors();
    expect(error.getDisabledAt()).to.deep.equal(stateTransition.publicKeysDisabledAt);
    expect(error.getTimeWindowStart()).to.deep.equal(timeWindowStart);
    expect(error.getTimeWindowEnd()).to.deep.equal(timeWindowEnd);
  });

  it('should throw InvalidIdentityPublicKeyIdError', async () => {
    stateTransition.setPublicKeyIdsToDisable([3]);
    stateTransition.setPublicKeysDisabledAt(new Date());

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expectValidationError(result, InvalidIdentityPublicKeyIdError);

    const [error] = result.getErrors();
    expect(error.getId()).to.equal(3);
  });

  it('should pass when disable public key', async () => {
    stateTransition.setPublicKeyIdsToDisable([1]);
    stateTransition.setPublicKeysDisabledAt(new Date());
    stateTransition.setPublicKeysToAdd(undefined);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchIdentity)
      .to.be.calledOnceWithExactly(stateTransition.getIdentityId());

    expect(stateRepositoryMock.fetchLatestPlatformBlockHeader)
      .to.be.calledOnce();
  });

  it('should pass when add public key', async () => {
    stateTransition.setPublicKeyIdsToDisable(undefined);
    stateTransition.setPublicKeysDisabledAt(undefined);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchIdentity)
      .to.be.calledOnceWithExactly(stateTransition.getIdentityId());

    expect(stateRepositoryMock.fetchLatestPlatformBlockHeader)
      .to.not.be.called();

    expect(validatePublicKeysMock)
      .to.be.calledOnceWithExactly(
        [...identity.getPublicKeys(), ...stateTransition.getPublicKeysToAdd()]
          .map((pk) => pk.toObject()),
      );
  });

  it('should pass when add and disable public keys', async () => {
    stateTransition.setPublicKeyIdsToDisable([1]);
    stateTransition.setPublicKeysDisabledAt(new Date());

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchIdentity)
      .to.be.calledOnceWithExactly(stateTransition.getIdentityId());

    expect(stateRepositoryMock.fetchLatestPlatformBlockHeader)
      .to.be.calledOnce();

    expect(stateRepositoryMock.fetchLatestPlatformBlockHeader)
      .to.be.calledOnce();
  });

  it('should not be able to add public key with disabledAt field', async () => {
    const publicKeysError = new SomeConsensusError('test');
    const publicKeysResult = new ValidationResult([
      publicKeysError,
    ]);

    validatePublicKeysAreEnabledMock.returns(publicKeysResult);

    const result = await validateIdentityUpdateTransitionState(
      stateTransition,
    );

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(publicKeysError);

    expect(validatePublicKeysAreEnabledMock).to.be.calledOnceWithExactly(
      stateTransition.addPublicKeys.map((pk) => pk.toObject()),
    );
  });
});
