const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const ValidationResult = require('@dashevo/dpp/lib/validation/ValidationResult');
const createMasternodeIdentityFactory = require('../../../../lib/identity/masternode/createMasternodeIdentityFactory');
const InvalidMasternodeIdentityError = require('../../../../lib/identity/masternode/errors/InvalidMasternodeIdentityError');

describe('createMasternodeIdentityFactory', () => {
  let createMasternodeIdentity;
  let dppMock;
  let stateRepositoryMock;
  let validationResult;

  beforeEach(function beforeEach() {
    dppMock = createDPPMock(this.sinon);
    stateRepositoryMock = createStateRepositoryMock(this.sinon);

    validationResult = new ValidationResult();

    dppMock.identity.validate.resolves(validationResult);

    createMasternodeIdentity = createMasternodeIdentityFactory(
      dppMock,
      stateRepositoryMock,
    );
  });

  it('should create masternode identity', async () => {
    const identityId = generateRandomIdentifier();
    const pubKeyData = Buffer.from([0]);
    const pubKeyType = IdentityPublicKey.TYPES.ECDSA_HASH160;

    await createMasternodeIdentity(identityId, pubKeyData, pubKeyType, false);

    const identity = new Identity({
      protocolVersion: dppMock.getProtocolVersion(),
      id: identityId,
      publicKeys: [{
        id: 0,
        type: pubKeyType,
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readOnly: true,
        // Copy data buffer
        data: Buffer.from([0]),
      }],
      balance: 0,
      revision: 0,
    });

    expect(stateRepositoryMock.storeIdentity).to.have.been.calledOnceWithExactly(identity);

    const publicKeyHashes = await Promise.all(
      identity
        .getPublicKeys()
        .map((publicKey) => publicKey.hash()),
    );

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.have.been.calledOnceWithExactly(
      identity.getId(),
      publicKeyHashes,
    );

    expect(dppMock.identity.validate).to.be.calledOnceWithExactly(identity);
  });

  it('should store identity and public key hashed to the previous store', async () => {
    const identityId = generateRandomIdentifier();
    const pubKeyData = Buffer.from([0]);
    const pubKeyType = IdentityPublicKey.TYPES.ECDSA_HASH160;

    await createMasternodeIdentity(identityId, pubKeyData, pubKeyType, true);

    const identity = new Identity({
      protocolVersion: dppMock.getProtocolVersion(),
      id: identityId,
      publicKeys: [{
        id: 0,
        type: pubKeyType,
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readOnly: true,
        // Copy data buffer
        data: Buffer.from([0]),
      }],
      balance: 0,
      revision: 0,
    });

    expect(stateRepositoryMock.storeIdentity).to.have.been.calledOnceWithExactly(identity);

    const publicKeyHashes = await Promise.all(
      identity
        .getPublicKeys()
        .map(async (publicKey) => publicKey.hash()),
    );

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.have.been.calledOnceWithExactly(
      identity.getId(),
      publicKeyHashes,
    );

    expect(dppMock.identity.validate).to.be.calledOnceWithExactly(identity);
  });

  it('should throw DPPValidationAbciError if identity is not valid', async () => {
    const validationError = new Error('Validation error');

    validationResult.addError(validationError);

    const identityId = generateRandomIdentifier();
    const pubKeyData = Buffer.from([0]);
    const pubKeyType = IdentityPublicKey.TYPES.ECDSA_HASH160;

    try {
      await createMasternodeIdentity(identityId, pubKeyData, pubKeyType, false);

      expect.fail('should fail with an error');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidMasternodeIdentityError);
      expect(e.message).to.be.equal('Invalid masternode identity');
      expect(e.getValidationError()).to.be.deep.equal(validationError);
    }
  });
});
