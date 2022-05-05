const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const ValidationResult = require('@dashevo/dpp/lib/validation/ValidationResult');
const Address = require('@dashevo/dashcore-lib/lib/address');
const Script = require('@dashevo/dashcore-lib/lib/script');
const createMasternodeIdentityFactory = require('../../../../lib/identity/masternode/createMasternodeIdentityFactory');
const InvalidMasternodeIdentityError = require('../../../../lib/identity/masternode/errors/InvalidMasternodeIdentityError');

describe('createMasternodeIdentityFactory', () => {
  let createMasternodeIdentity;
  let dppMock;
  let stateRepositoryMock;
  let validationResult;
  let getWithdrawPubKeyTypeFromPayoutScriptMock;
  let getPublicKeyFromPayoutScriptMock;

  beforeEach(function beforeEach() {
    dppMock = createDPPMock(this.sinon);
    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    getWithdrawPubKeyTypeFromPayoutScriptMock = this.sinon.stub().returns(
      IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH,
    );

    getPublicKeyFromPayoutScriptMock = this.sinon.stub().returns(
      Buffer.alloc(20, 1),
    );

    validationResult = new ValidationResult();

    dppMock.identity.validate.resolves(validationResult);

    createMasternodeIdentity = createMasternodeIdentityFactory(
      dppMock,
      stateRepositoryMock,
      getWithdrawPubKeyTypeFromPayoutScriptMock,
      getPublicKeyFromPayoutScriptMock,
    );
  });

  it('should create masternode identity', async () => {
    const identityId = generateRandomIdentifier();
    const pubKeyData = Buffer.from([0]);
    const pubKeyType = IdentityPublicKey.TYPES.ECDSA_HASH160;

    await createMasternodeIdentity(identityId, pubKeyData, pubKeyType);

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
    expect(getWithdrawPubKeyTypeFromPayoutScriptMock).to.not.be.called();
    expect(getPublicKeyFromPayoutScriptMock).to.not.be.called();

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

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

    await createMasternodeIdentity(identityId, pubKeyData, pubKeyType);

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

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

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
      await createMasternodeIdentity(identityId, pubKeyData, pubKeyType);

      expect.fail('should fail with an error');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidMasternodeIdentityError);
      expect(e.message).to.be.equal('Invalid masternode identity');
      expect(e.getValidationError()).to.be.deep.equal(validationError);
    }
  });

  it('should create masternode identity with payoutScript public key', async () => {
    const identityId = generateRandomIdentifier();
    const pubKeyData = Buffer.from([0]);
    const pubKeyType = IdentityPublicKey.TYPES.ECDSA_HASH160;
    const payoutScript = new Script(Address.fromString('7UkJidhNjEPJCQnCTXeaJKbJmL4JuyV66w'));

    await createMasternodeIdentity(identityId, pubKeyData, pubKeyType, payoutScript);

    const identity = new Identity({
      protocolVersion: dppMock.getProtocolVersion(),
      id: identityId,
      publicKeys: [{
        id: 0,
        type: pubKeyType,
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readOnly: true,
        data: Buffer.from([0]),
      }, {
        id: 1,
        type: IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH,
        purpose: IdentityPublicKey.PURPOSES.WITHDRAW,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.CRITICAL,
        readOnly: false,
        data: Buffer.alloc(20, 1),
      }],
      balance: 0,
      revision: 0,
    });

    expect(stateRepositoryMock.storeIdentity).to.have.been.calledOnceWithExactly(identity);
    expect(getWithdrawPubKeyTypeFromPayoutScriptMock).to.be.calledOnce();
    expect(getPublicKeyFromPayoutScriptMock).to.be.calledOnce();

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.have.been.calledOnceWithExactly(
      identity.getId(),
      publicKeyHashes,
    );

    expect(dppMock.identity.validate).to.be.calledOnceWithExactly(identity);
  });
});
