const identitySchema = require('../../../../../../../../rs-dpp/src/schema/identity/v0/identity.json');
const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');
const getIdentityFixture = require('../../../../../../../lib/test/fixtures/getIdentityFixture');
const getIdentityUpdateTransitionFixture = require('../../../../../../../lib/test/fixtures/getIdentityUpdateTransitionFixture');

const { default: loadWasmDpp } = require('../../../../../../..');
const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError');
const generateRandomIdentifierAsync = require('../../../../../../../lib/test/utils/generateRandomIdentifierAsync');
const getBlsAdapterMock = require('../../../../../../../lib/test/mocks/getBlsAdapterMock');

describe.skip('validateIdentityUpdateTransitionStateFactory', () => {
  let validateIdentityUpdateTransitionState;
  let stateRepositoryMock;
  let stateTransition;
  let identity;
  let rawIdentity;
  let blockTime;
  let executionContext;

  let Identity;
  let IdentityPublicKey;
  let InvalidIdentityRevisionError;
  let IdentityPublicKeyIsReadOnlyError;
  let IdentityPublicKeyIsDisabledError;
  let InvalidIdentityPublicKeyIdError;
  let MissingMasterPublicKeyError;
  let MaxIdentityPublicKeyLimitReachedError;
  let StateTransitionExecutionContext;
  let IdentityUpdateTransitionStateValidator;

  before(async () => {
    ({
      Identity,
      IdentityPublicKey,
      InvalidIdentityRevisionError,
      IdentityPublicKeyIsReadOnlyError,
      IdentityPublicKeyIsDisabledError,
      InvalidIdentityPublicKeyIdError,
      MaxIdentityPublicKeyLimitReachedError,
      StateTransitionExecutionContext,
      IdentityUpdateTransitionStateValidator,
      MissingMasterPublicKeyError,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    rawIdentity = (await getIdentityFixture()).toObject();
    // Patch identity id to be acceptable by wasm-dpp
    rawIdentity.id = await generateRandomIdentifierAsync();
    identity = new Identity(rawIdentity);

    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    blockTime = Date.now();
    const blsAdapter = await getBlsAdapterMock();

    stateRepositoryMock.fetchLatestPlatformBlockTime = this.sinon.stub();
    stateRepositoryMock.fetchLatestPlatformBlockTime.resolves(blockTime);

    executionContext = new StateTransitionExecutionContext();
    const validator = new IdentityUpdateTransitionStateValidator(stateRepositoryMock, blsAdapter);
    validateIdentityUpdateTransitionState = (st) => validator.validate(st, executionContext);

    stateTransition = await getIdentityUpdateTransitionFixture();

    stateTransition.setRevision(identity.getRevision() + 1);
    stateTransition.setPublicKeyIdsToDisable(undefined);

    const privateKey = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';

    await stateTransition.signByPrivateKey(Buffer.from(privateKey, 'hex'), IdentityPublicKey.TYPES.ECDSA_SECP256K1);
  });

  it('should return InvalidIdentityRevisionError if new revision is not incremented by 1', async () => {
    stateTransition.setRevision(rawIdentity.revision + 2);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    await expectValidationError(result, InvalidIdentityRevisionError);

    const [error] = result.getErrors();
    expect(error.getIdentityId()).to.deep.equal(stateTransition.getIdentityId().toBuffer());
    expect(error.getCurrentRevision()).to.equal(rawIdentity.revision);
  });

  it('should return IdentityPublicKeyIsReadOnlyError if disabling public key is readOnly', async () => {
    const keys = identity.getPublicKeys();
    keys[0].setReadOnly(true);
    identity.setPublicKeys(keys);

    stateTransition.setPublicKeyIdsToDisable([0]);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    await expectValidationError(result, IdentityPublicKeyIsReadOnlyError);

    const [error] = result.getErrors();
    expect(error.getPublicKeyIndex()).to.equal(0);
  });

  it('should return IdentityPublicKeyIsDisabledError if disabling public key is already disabled', async () => {
    const keys = identity.getPublicKeys();
    keys[0].setDisabledAt(new Date());
    identity.setPublicKeys(keys);
    stateTransition.setPublicKeyIdsToDisable([0]);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    await expectValidationError(result, IdentityPublicKeyIsDisabledError);

    const [error] = result.getErrors();
    expect(error.getPublicKeyIndex()).to.equal(0);
  });

  it('should throw InvalidIdentityPublicKeyIdError if identity does not contain public key with disabling ID', async () => {
    stateTransition.setPublicKeyIdsToDisable([3]);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    await expectValidationError(result, InvalidIdentityPublicKeyIdError);

    const [error] = result.getErrors();
    expect(error.getId()).to.equal(3);
  });

  it('should pass when disabling public key', async function () {
    stateTransition.setPublicKeyIdsToDisable([1]);
    stateTransition.setPublicKeysToAdd(undefined);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expect(result.isValid()).to.be.true();

    const { match } = this.sinon;
    expect(stateRepositoryMock.fetchIdentity)
      .to.be.calledOnceWithExactly(
        match((val) => val.toBuffer().equals(stateTransition.getIdentityId().toBuffer())),
        match.instanceOf(StateTransitionExecutionContext),
      );

    expect(stateRepositoryMock.fetchLatestPlatformBlockTime)
      .to.be.calledOnce();
  });

  it('should pass when adding public key', async function () {
    stateTransition.setPublicKeyIdsToDisable(undefined);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expect(result.isValid()).to.be.true();

    const { match } = this.sinon;
    expect(stateRepositoryMock.fetchIdentity)
      .to.be.calledOnceWithExactly(
        match((val) => val.toBuffer().equals(stateTransition.getIdentityId().toBuffer())),
        match.instanceOf(StateTransitionExecutionContext),
      );

    expect(stateRepositoryMock.fetchLatestPlatformBlockTime)
      .to.not.be.called();
  });

  it('should pass when both adding and disabling public keys', async function () {
    stateTransition.setPublicKeyIdsToDisable([1]);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    expect(result.isValid()).to.be.true();

    const { match } = this.sinon;
    expect(stateRepositoryMock.fetchIdentity)
      .to.be.calledOnceWithExactly(
        match((val) => val.toBuffer().equals(stateTransition.getIdentityId().toBuffer())),
        match.instanceOf(StateTransitionExecutionContext),
      );

    expect(stateRepositoryMock.fetchLatestPlatformBlockTime)
      .to.be.calledOnce();
  });

  it('should validate purpose and security level', async () => {
    const identityKeys = identity.getPublicKeys();
    identityKeys.forEach((key) => key.setSecurityLevel(1));
    identity.setPublicKeys(identityKeys);

    const keysToAdd = stateTransition.getPublicKeysToAdd();
    keysToAdd.forEach((key) => key.setSecurityLevel(1));
    stateTransition.setPublicKeysToAdd(keysToAdd);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    await expectValidationError(result, MissingMasterPublicKeyError);
  });

  it('should validate public keys to add', async () => {
    // Reach max allowed public keys to fail validation
    const { maxItems } = identitySchema.properties.publicKeys;

    const firstKey = identity.getPublicKeys()[0].toObject();
    const keys = Array.from({ length: maxItems + 1 })
      .map((_, index) => new IdentityPublicKey({ ...firstKey, id: index }));
    identity.setPublicKeys(keys);

    const result = await validateIdentityUpdateTransitionState(stateTransition);

    await expectValidationError(result, MaxIdentityPublicKeyLimitReachedError);
  });

  // TODO: remove?
  // Skipped, because two tests above are seem to be enough
  it.skip('should validate resulting identity public keys', async () => {
    // const publicKeysError = new SomeConsensusError('test');
    //
    // validatePublicKeysMock.returns(new ValidationResult([publicKeysError]));
    //
    // const result = await validateIdentityUpdateTransitionState(stateTransition);
    //
    // expectValidationError(result, SomeConsensusError);
    //
    // expect(validatePublicKeysMock).to.be.calledOnce();
    //
    // const publicKeys = [...identity.getPublicKeys(), ...stateTransition.getPublicKeysToAdd()];
    //
    // expect(validatePublicKeysMock).to.be.calledWithExactly(
    //   publicKeys.map((pk) => pk.toObject()),
    // );
  });

  it('should return valid result on dry run', async function () {
    stateTransition.setPublicKeyIdsToDisable([3]);

    // Make code that executes after dry run check to fail
    stateRepositoryMock.fetchLatestPlatformBlockTime.resolves({});

    executionContext.enableDryRun();
    const result = await validateIdentityUpdateTransitionState(stateTransition);

    executionContext.disableDryRun();

    expect(result.isValid()).to.be.true();

    const { match } = this.sinon;
    expect(stateRepositoryMock.fetchIdentity)
      .to.be.calledOnceWithExactly(
        match((val) => val.toBuffer().equals(stateTransition.getIdentityId().toBuffer())),
        match.instanceOf(StateTransitionExecutionContext),
      );

    expect(stateRepositoryMock.fetchLatestPlatformBlockTime)
      .to.not.be.called();
  });
});
