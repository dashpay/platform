const { expectValidationError } = require('../../../../lib/test/expect/expectError');
const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const getIdentityFixture = require('../../../../lib/test/fixtures/getIdentityFixture');
const getPrivateAndPublicKeyForSigningFixture = require('../../../../lib/test/fixtures/getPrivateAndPublicKeyForSigningFixture');

const { default: loadWasmDpp } = require('../../../..');
let {
  DashPlatformProtocol,
  validateStateTransitionIdentitySignature: validate,
  ValidationResult,
  StateTransitionExecutionContext,
  IdentityNotFoundError,
  MissingPublicKeyError,
  InvalidIdentityPublicKeyTypeError,
  InvalidStateTransitionSignatureError,
  InvalidSignaturePublicKeySecurityLevelError,
  PublicKeySecurityLevelNotMetError,
  WrongPublicKeyPurposeError,
  PublicKeyIsDisabledError,
  IdentityPublicKey,
} = require('../../../..');
const getBlsMock = require('../../../../lib/test/mocks/getBlsAdapterMock');
const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');

describe.skip('validateStateTransitionIdentitySignatureFactory', () => {
  let stateTransition;
  let ownerId;
  let identity;
  let identityPublicKey;
  let publicKeyId;
  let executionContext;
  let stateRepositoryMock;
  let dpp;
  let validateStateTransitionIdentitySignature;
  let privateKey;

  beforeEach(async function beforeEach() {
    ({
      DashPlatformProtocol,
      ValidationResult,
      validateStateTransitionIdentitySignature: validate,
      StateTransitionExecutionContext,
      IdentityNotFoundError,
      MissingPublicKeyError,
      InvalidIdentityPublicKeyTypeError,
      InvalidStateTransitionSignatureError,
      InvalidSignaturePublicKeySecurityLevelError,
      PublicKeySecurityLevelNotMetError,
      WrongPublicKeyPurposeError,
      PublicKeyIsDisabledError,
      IdentityPublicKey,
    } = await loadWasmDpp());
    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchDataContract.resolves();
    const blsMock = getBlsMock();

    dpp = new DashPlatformProtocol(blsMock, stateRepositoryMock);

    executionContext = new StateTransitionExecutionContext();

    publicKeyId = 2;

    identity = await getIdentityFixture();
    ownerId = identity.getId();
    const dataContract = await getDataContractFixture(identity.getId());

    stateTransition = await dpp.dataContract.createDataContractCreateTransition(dataContract);

    ({ privateKey, identityPublicKey } = await getPrivateAndPublicKeyForSigningFixture());
    identity.addPublicKey(identityPublicKey);

    stateTransition.sign(identity.getPublicKeyById(2), privateKey, getBlsMock());

    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    validateStateTransitionIdentitySignature = (st) => validate(
      stateRepositoryMock,
      st,
      executionContext,
      blsMock,
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

    // For this validator we always use default execution context,
    // as there's no dry run
    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnce();
    expect(
      stateRepositoryMock.fetchIdentity.getCall(0).args[0].toBuffer(),
    ).to.be.deep.equal(
      ownerId.toBuffer(),
    );
  });

  it('should return invalid result if owner id doesn\'t exist', async () => {
    stateRepositoryMock.fetchIdentity.resolves(undefined);

    const result = await validateStateTransitionIdentitySignature(
      stateTransition,
    );

    await expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.be.an.instanceOf(IdentityNotFoundError);
    expect(error.getCode()).to.equal(2000);
    expect(error.getIdentityId()).to.deep.equal(identity.getId().toBuffer());

    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnce();
    expect(
      stateRepositoryMock.fetchIdentity.getCall(0).args[0].toBuffer(),
    ).to.be.deep.equal(
      ownerId.toBuffer(),
    );
  });

  it("should return MissingPublicKeyError if the identity doesn't have a matching public key", async () => {
    const { identityPublicKey: publicKey } = await getPrivateAndPublicKeyForSigningFixture();
    publicKey.setId(99);
    identity.setPublicKeys([publicKey]);

    const result = await validateStateTransitionIdentitySignature(
      stateTransition,
    );

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.false();
    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnce();
    expect(
      stateRepositoryMock.fetchIdentity.getCall(0).args[0].toBuffer(),
    ).to.be.deep.equal(
      ownerId.toBuffer(),
    );

    expect(result.getErrors()).to.be.an('array');
    expect(result.getErrors()).to.have.lengthOf(1);

    const [error] = result.getErrors();

    expect(error).to.be.instanceOf(MissingPublicKeyError);
    expect(error.getPublicKeyId()).to.equal(publicKeyId);
  });

  it('should return InvalidIdentityPublicKeyTypeError if type does not exist', async () => {
    // It is physically impossible to create an identity with the type out
    // of allowed type range, since rust implementation uses enum, and it's
    // impossible to create an enum from an invalid key type
    const type = IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH;
    const publicKeys = identity.getPublicKeys();
    publicKeys[2].setType(type);
    identity.setPublicKeys(publicKeys);

    const result = await validateStateTransitionIdentitySignature(
      stateTransition,
    );

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.false();
    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnce();
    expect(
      stateRepositoryMock.fetchIdentity.getCall(0).args[0].toBuffer(),
    ).to.be.deep.equal(
      ownerId.toBuffer(),
    );

    expect(result.getErrors()).to.be.an('array');
    expect(result.getErrors()).to.have.lengthOf(1);

    const [error] = result.getErrors();

    expect(error).to.be.instanceOf(InvalidIdentityPublicKeyTypeError);
    expect(error.getPublicKeyType()).to.equal(type);
  });

  it('should return InvalidStateTransitionSignatureError if signature is invalid', async () => {
    const publicKeys = identity.getPublicKeys();
    publicKeys[2].setData(Buffer.from('00'.repeat(32), 'hex'));
    identity.setPublicKeys(publicKeys);

    const result = await validateStateTransitionIdentitySignature(
      stateTransition,
    );

    expect(result).to.be.instanceOf(ValidationResult);

    expect(result.isValid()).to.be.false();
    expect(result.getErrors()).to.be.an('array');
    expect(result.getErrors()).to.have.lengthOf(1);

    const [error] = result.getErrors();

    expect(error).to.be.instanceOf(InvalidStateTransitionSignatureError);

    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnce();
    expect(
      stateRepositoryMock.fetchIdentity.getCall(0).args[0].toBuffer(),
    ).to.be.deep.equal(
      ownerId.toBuffer(),
    );
  });

  describe('Consensus errors', () => {
    it('should return InvalidSignaturePublicKeySecurityLevelConsensusError if InvalidSignaturePublicKeySecurityLevelError was thrown', async () => {
      const publicKeys = identity.getPublicKeys();
      publicKeys[2].setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MASTER);
      identity.setPublicKeys(publicKeys);

      const result = await validateStateTransitionIdentitySignature(
        stateTransition,
      );

      expect(result).to.be.instanceOf(ValidationResult);

      expect(result.isValid()).to.be.false();
      expect(result.getErrors()).to.be.an('array');
      expect(result.getErrors()).to.have.lengthOf(1);

      const [error] = result.getErrors();

      expect(error).to.be.instanceOf(InvalidSignaturePublicKeySecurityLevelError);
      expect(error.getPublicKeySecurityLevel()).to.equal(IdentityPublicKey.SECURITY_LEVELS.MASTER);
      expect(error.getKeySecurityLevelRequirement()).to.deep.equal([2]);
    });

    // TODO: the error is not used anymore,
    //  remove the test and remaining `match` variants in rs-dpp
    //  that still expecting to receive this error?
    it.skip('should return PublicKeySecurityLevelNotMetConsensusError if PublicKeySecurityLevelNotMetError was thrown', async () => {
      const publicKeys = identity.getPublicKeys();
      publicKeys[2].setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MEDIUM);
      identity.setPublicKeys(publicKeys);

      const result = await validateStateTransitionIdentitySignature(
        stateTransition,
      );

      expect(result).to.be.instanceOf(ValidationResult);

      expect(result.isValid()).to.be.false();
      expect(result.getErrors()).to.be.an('array');
      expect(result.getErrors()).to.have.lengthOf(1);

      const [error] = result.getErrors();
      expect(error).to.be.instanceOf(PublicKeySecurityLevelNotMetError);
      expect(error.getPublicKeySecurityLevel()).to.equal(3);
      expect(error.getKeySecurityLevelRequirement()).to.equal(2);
    });

    it('should return WrongPublicKeyPurposeConsensusError if WrongPublicKeyPurposeError was thrown', async () => {
      const publicKeys = identity.getPublicKeys();
      publicKeys[2].setPurpose(IdentityPublicKey.PURPOSES.ENCRYPTION);
      identity.setPublicKeys(publicKeys);

      const result = await validateStateTransitionIdentitySignature(
        stateTransition,
      );

      expect(result).to.be.instanceOf(ValidationResult);

      expect(result.isValid()).to.be.false();
      expect(result.getErrors()).to.be.an('array');
      expect(result.getErrors()).to.have.lengthOf(1);

      const [error] = result.getErrors();
      expect(error).to.be.instanceOf(WrongPublicKeyPurposeError);

      expect(error.getPublicKeyPurpose()).to.equal(IdentityPublicKey.PURPOSES.ENCRYPTION);
      expect(error.getKeyPurposeRequirement()).to.equal(IdentityPublicKey.PURPOSES.AUTHENTICATION);
    });

    it('should return PublicKeyIsDisabledConsensusError if PublicKeyIsDisabledError was thrown', async () => {
      const publicKeys = identity.getPublicKeys();
      publicKeys[2].setDisabledAt(new Date());
      identity.setPublicKeys(publicKeys);

      const result = await validateStateTransitionIdentitySignature(
        stateTransition,
      );

      expect(result).to.be.instanceOf(ValidationResult);

      expect(result.isValid()).to.be.false();
      expect(result.getErrors()).to.be.an('array');
      expect(result.getErrors()).to.have.lengthOf(1);

      const [error] = result.getErrors();
      expect(error).to.be.instanceOf(PublicKeyIsDisabledError);
      expect(error.getPublicKeyId()).to.deep.equal(publicKeyId);
    });

    it('should return InvalidStateTransitionSignatureError if DPPError was thrown', async () => {
      const publicKeys = identity.getPublicKeys();
      publicKeys[2].setData(Buffer.from('00'.repeat(32), 'hex'));
      identity.setPublicKeys(publicKeys);

      const result = await validateStateTransitionIdentitySignature(
        stateTransition,
      );

      expect(result).to.be.instanceOf(ValidationResult);

      expect(result.isValid()).to.be.false();
      expect(result.getErrors()).to.be.an('array');
      expect(result.getErrors()).to.have.lengthOf(1);

      const [error] = result.getErrors();
      expect(error).to.be.instanceOf(InvalidStateTransitionSignatureError);
    });

    it('should not verify signature on dry run', async () => {
      // This will produce an error during signature validation
      const publicKeys = identity.getPublicKeys();
      publicKeys[2].setDisabledAt(new Date());
      identity.setPublicKeys(publicKeys);

      executionContext.enableDryRun();

      const result = await validateStateTransitionIdentitySignature(
        stateTransition,
      );

      executionContext.disableDryRun();

      expect(result.isValid()).to.be.true();
      expect(result.getErrors()).to.be.an('array');
      expect(result.getErrors()).to.be.empty();
    });
  });
});
