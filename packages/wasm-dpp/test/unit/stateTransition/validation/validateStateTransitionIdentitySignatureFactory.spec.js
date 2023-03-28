const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

const PublicKeyIsDisabledConsensusError = require('@dashevo/dpp/lib/errors/consensus/signature/PublicKeyIsDisabledError');
const WrongPublicKeyPurposeConsensusError = require('@dashevo/dpp/lib/errors/consensus/signature/WrongPublicKeyPurposeError');
const PublicKeySecurityLevelNotMetConsensusError = require('@dashevo/dpp/lib/errors/consensus/signature/PublicKeySecurityLevelNotMetError');
const InvalidSignaturePublicKeySecurityLevelConsensusError = require('@dashevo/dpp/lib/errors/consensus/signature/InvalidSignaturePublicKeySecurityLevelError');
const InvalidSignaturePublicKeySecurityLevelError = require('@dashevo/dpp/lib/stateTransition/errors/InvalidSignaturePublicKeySecurityLevelError');
const PublicKeySecurityLevelNotMetError = require('@dashevo/dpp/lib/stateTransition/errors/PublicKeySecurityLevelNotMetError');
const WrongPublicKeyPurposeError = require('@dashevo/dpp/lib/stateTransition/errors/WrongPublicKeyPurposeError');
const PublicKeyIsDisabledError = require('@dashevo/dpp/lib/stateTransition/errors/PublicKeyIsDisabledError');
const DPPError = require('@dashevo/dpp/lib/errors/DPPError');

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
} = require('../../../..');
const getBlsMock = require('../../../../lib/test/mocks/getBlsAdapterMock');
const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');

describe('validateStateTransitionIdentitySignatureFactory', () => {
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
    } = await loadWasmDpp());
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
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
    stateTransition.setExecutionContext(executionContext);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    validateStateTransitionIdentitySignature = (st) => validate(stateRepositoryMock, st, blsMock);
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
    // stateTransition.verifySignature.resolves(false);
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
      const e = new InvalidSignaturePublicKeySecurityLevelError(1, 0);

      stateTransition.verifySignature.throws(e);

      const result = await validateStateTransitionIdentitySignature(
        stateTransition,
      );

      expect(result).to.be.instanceOf(ValidationResult);

      expect(result.isValid()).to.be.false();
      expect(result.getErrors()).to.be.an('array');
      expect(result.getErrors()).to.have.lengthOf(1);

      const [error] = result.getErrors();
      expect(error).to.be.instanceOf(InvalidSignaturePublicKeySecurityLevelConsensusError);
      expect(error.getPublicKeySecurityLevel()).to.equal(1);
      expect(error.getKeySecurityLevelRequirement()).to.equal(0);
    });

    it('should return PublicKeySecurityLevelNotMetConsensusError if PublicKeySecurityLevelNotMetError was thrown', async () => {
      const e = new PublicKeySecurityLevelNotMetError(1, 2);

      stateTransition.verifySignature.throws(e);

      const result = await validateStateTransitionIdentitySignature(
        stateTransition,
      );

      expect(result).to.be.instanceOf(ValidationResult);

      expect(result.isValid()).to.be.false();
      expect(result.getErrors()).to.be.an('array');
      expect(result.getErrors()).to.have.lengthOf(1);

      const [error] = result.getErrors();
      expect(error).to.be.instanceOf(PublicKeySecurityLevelNotMetConsensusError);
      expect(error.getPublicKeySecurityLevel()).to.equal(1);
      expect(error.getKeySecurityLevelRequirement()).to.equal(2);
    });

    it('should return WrongPublicKeyPurposeConsensusError if WrongPublicKeyPurposeError was thrown', async () => {
      const e = new WrongPublicKeyPurposeError(4, 2);

      stateTransition.verifySignature.throws(e);

      const result = await validateStateTransitionIdentitySignature(
        stateTransition,
      );

      expect(result).to.be.instanceOf(ValidationResult);

      expect(result.isValid()).to.be.false();
      expect(result.getErrors()).to.be.an('array');
      expect(result.getErrors()).to.have.lengthOf(1);

      const [error] = result.getErrors();
      expect(error).to.be.instanceOf(WrongPublicKeyPurposeConsensusError);

      expect(error.getPublicKeyPurpose()).to.equal(4);
      expect(error.getKeyPurposeRequirement()).to.equal(2);
    });

    it('should return PublicKeyIsDisabledConsensusError if PublicKeyIsDisabledError was thrown', async () => {
      const e = new PublicKeyIsDisabledError(identityPublicKey);

      stateTransition.verifySignature.throws(e);

      const result = await validateStateTransitionIdentitySignature(
        stateTransition,
      );

      expect(result).to.be.instanceOf(ValidationResult);

      expect(result.isValid()).to.be.false();
      expect(result.getErrors()).to.be.an('array');
      expect(result.getErrors()).to.have.lengthOf(1);

      const [error] = result.getErrors();
      expect(error).to.be.instanceOf(PublicKeyIsDisabledConsensusError);
      expect(error.getPublicKeyId()).to.deep.equal(publicKeyId);
    });

    it('should return InvalidStateTransitionSignatureError if DPPError was thrown', async () => {
      const e = new DPPError('Dpp error');

      stateTransition.verifySignature.throws(e);

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

    it('should throw unknown error', async () => {
      const e = new Error('unknown error');

      stateTransition.verifySignature.throws(e);

      try {
        await validateStateTransitionIdentitySignature(
          stateTransition,
        );

        expect.fail('should throw an error');
      } catch (error) {
        expect(error).to.equal(e);
      }
    });

    it('should not verify signature on dry run', async () => {
      const e = new DPPError('Dpp error');

      stateTransition.verifySignature.throws(e);

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
