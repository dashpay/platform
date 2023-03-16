const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const Address = require('@dashevo/dashcore-lib/lib/address');
const Script = require('@dashevo/dashcore-lib/lib/script');
const createMasternodeIdentityFactory = require('../../../../lib/identity/masternode/createMasternodeIdentityFactory');
const InvalidMasternodeIdentityError = require('../../../../lib/identity/masternode/errors/InvalidMasternodeIdentityError');
const BlockInfo = require('../../../../lib/blockExecution/BlockInfo');

describe('createMasternodeIdentityFactory', () => {
  let createMasternodeIdentity;
  let dppMock;
  let validationResult;
  let getWithdrawPubKeyTypeFromPayoutScriptMock;
  let getPublicKeyFromPayoutScriptMock;
  let identityRepositoryMock;
  let blockInfo;
  let Identity;
  let IdentityPublicKey;
  let ValidationResult;
  let KeyPurpose;
  let KeyType;
  let KeySecurityLevel;

  before(function before() {
    ({ Identity, IdentityPublicKey, ValidationResult, KeyPurpose, KeyType, KeySecurityLevel } = this.dppWasm);
  });

  beforeEach(function beforeEach() {
    dppMock = createDPPMock(this.sinon);

    getWithdrawPubKeyTypeFromPayoutScriptMock = this.sinon.stub().returns(
      KeyType.BIP13_SCRIPT_HASH,
    );

    getPublicKeyFromPayoutScriptMock = this.sinon.stub().returns(
      Buffer.alloc(20, 1),
    );

    validationResult = new ValidationResult();

    dppMock.identity.validate.resolves(validationResult);

    identityRepositoryMock = {
      create: this.sinon.stub(),
    };

    createMasternodeIdentity = createMasternodeIdentityFactory(
      dppMock,
      identityRepositoryMock,
      getWithdrawPubKeyTypeFromPayoutScriptMock,
      getPublicKeyFromPayoutScriptMock,
      this.dppWasm
    );

    blockInfo = new BlockInfo(1, 0, Date.now());
  });

  it('should create masternode identity', async () => {
    const identityId = generateRandomIdentifier();
    const pubKeyData = Buffer.from([0]);
    const pubKeyType = KeyType.ECDSA_HASH160;

    const result = await createMasternodeIdentity(this.dppWasm, blockInfo, identityId, pubKeyData, pubKeyType);

    const identity = new Identity({
      protocolVersion: 1,
      id: identityId,
      publicKeys: [{
        id: 0,
        type: pubKeyType,
        purpose: KeyPurpose.AUTHENTICATION,
        securityLevel: KeySecurityLevel.MASTER,
        readOnly: true,
        // Copy data buffer
        data: Buffer.from([0]),
      }],
      balance: 0,
      revision: 0,
    });

    expect(result).to.deep.equal(identity);

    expect(identityRepositoryMock.create).to.have.been.calledOnceWithExactly(
      identity,
      blockInfo,
      { useTransaction: true },
    );

    expect(getWithdrawPubKeyTypeFromPayoutScriptMock).to.not.be.called();

    expect(getPublicKeyFromPayoutScriptMock).to.not.be.called();

    expect(dppMock.identity.validate).to.be.calledOnceWithExactly(identity);
  });

  it('should store identity and public key hashed to the previous store', async () => {
    const identityId = generateRandomIdentifier();
    const pubKeyData = Buffer.from([0]);
    const pubKeyType = KeyType.ECDSA_HASH160;

    const result = await createMasternodeIdentity(blockInfo, identityId, pubKeyData, pubKeyType);

    const identity = new Identity({
      protocolVersion: 1,
      id: identityId,
      publicKeys: [{
        id: 0,
        type: pubKeyType,
        purpose: KeyPurpose.AUTHENTICATION,
        securityLevel: KeySecurityLevel.MASTER,
        readOnly: true,
        // Copy data buffer
        data: Buffer.from([0]),
      }],
      balance: 0,
      revision: 0,
    });

    expect(result).to.deep.equal(identity);

    expect(identityRepositoryMock.create).to.have.been.calledOnceWithExactly(
      identity,
      blockInfo,
      { useTransaction: true },
    );
  });

  it('should throw DPPValidationAbciError if identity is not valid', async () => {
    const validationError = new Error('Validation error');

    validationResult.addError(validationError);

    const identityId = generateRandomIdentifier();
    const pubKeyData = Buffer.from([0]);
    const pubKeyType = KeyType.ECDSA_HASH160;

    try {
      await createMasternodeIdentity(blockInfo, identityId, pubKeyData, pubKeyType);

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
    const pubKeyType = KeyType.ECDSA_HASH160;
    const payoutScript = new Script(Address.fromString('7UkJidhNjEPJCQnCTXeaJKbJmL4JuyV66w'));

    const result = await createMasternodeIdentity(
      blockInfo,
      identityId,
      pubKeyData,
      pubKeyType,
      payoutScript,
    );

    const identity = new Identity({
      protocolVersion: 1,
      id: identityId,
      publicKeys: [{
        id: 0,
        type: pubKeyType,
        purpose: KeyPurpose.AUTHENTICATION,
        securityLevel: KeySecurityLevel.MASTER,
        readOnly: true,
        data: Buffer.from([0]),
      }, {
        id: 1,
        type: KeyType.BIP13_SCRIPT_HASH,
        purpose: KeyPurpose.WITHDRAW,
        securityLevel: KeySecurityLevel.CRITICAL,
        readOnly: false,
        data: Buffer.alloc(20, 1),
      }],
      balance: 0,
      revision: 0,
    });

    expect(result).to.deep.equal(identity);

    expect(identityRepositoryMock.create).to.have.been.calledOnceWithExactly(
      identity,
      blockInfo,
      { useTransaction: true },
    );

    expect(getWithdrawPubKeyTypeFromPayoutScriptMock).to.be.calledOnce();

    expect(getPublicKeyFromPayoutScriptMock).to.be.calledOnce();

    expect(dppMock.identity.validate).to.be.calledOnceWithExactly(identity);
  });
});
