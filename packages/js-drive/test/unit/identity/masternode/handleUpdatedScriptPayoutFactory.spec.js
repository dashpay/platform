// TODO: should we take it from other place?
const identitySchema = require('@dashevo/dpp/schema/identity/identity.json');
const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');
const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');

const Script = require('@dashevo/dashcore-lib/lib/script');
const handleUpdatedScriptPayoutFactory = require('../../../../lib/identity/masternode/handleUpdatedScriptPayoutFactory');
const BlockInfo = require('../../../../lib/blockExecution/BlockInfo');
const StorageResult = require('../../../../lib/storage/StorageResult');

describe('handleUpdatedScriptPayoutFactory', () => {
  let handleUpdatedScriptPayout;
  let getWithdrawPubKeyTypeFromPayoutScriptMock;
  let getPublicKeyFromPayoutScriptMock;
  let identity;
  let blockInfo;
  let identityRepositoryMock;
  let identityPublicKeyRepositoryMock;
  let IdentityPublicKey;
  let Identity;
  let KeyPurpose;
  let KeyType;
  let KeySecurityLevel;

  before(function before() {
    ({
      Identity, IdentityPublicKey, KeyPurpose, KeyType, KeySecurityLevel,
    } = this.dppWasm);
  });

  beforeEach(async function beforeEach() {
    identity = await getIdentityFixture();

    blockInfo = new BlockInfo(1, 0, Date.now());

    getWithdrawPubKeyTypeFromPayoutScriptMock = this.sinon.stub().returns(
      KeyType.ECDSA_HASH160,
    );

    getPublicKeyFromPayoutScriptMock = this.sinon.stub().returns(Buffer.alloc(20, '0'));

    identityRepositoryMock = {
      updateRevision: this.sinon.stub(),
      fetch: this.sinon.stub().resolves(new StorageResult(identity, [])),
    };

    identityPublicKeyRepositoryMock = {
      add: this.sinon.stub(),
      disable: this.sinon.stub(),
    };

    handleUpdatedScriptPayout = handleUpdatedScriptPayoutFactory(
      identityRepositoryMock,
      identityPublicKeyRepositoryMock,
      getWithdrawPubKeyTypeFromPayoutScriptMock,
      getPublicKeyFromPayoutScriptMock,
      this.dppWasm,
    );
  });

  it('should not update identity if identityPublicKeys max length was reached', async () => {
    const identifier = await generateRandomIdentifier();

    const { maxItems } = identitySchema.properties.publicKeys;

    const publicKeys = [];
    for (let i = 0; i <= maxItems; i++) {
      publicKeys.push({
        id: i,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readOnly: false,
      });
    }

    identity = await getIdentityFixture(identifier, publicKeys);

    identityRepositoryMock.fetch.resolves(new StorageResult(identity, []));

    const newPubKeyData = Buffer.alloc(20, '0');

    const result = await handleUpdatedScriptPayout(
      identity.getId(),
      newPubKeyData,
      blockInfo,
      identity.getPublicKeys()[0].getData(),
    );

    expect(result.createdEntities).to.have.lengthOf(0);
    expect(result.updatedEntities).to.have.lengthOf(0);
    expect(result.removedEntities).to.have.lengthOf(0);

    expect(identityRepositoryMock.updateRevision).to.not.be.called();
    expect(identityPublicKeyRepositoryMock.add).to.not.be.called();
    expect(identityPublicKeyRepositoryMock.disable).to.not.be.called();
  });

  it('should add a public key and disable an old one', async () => {
    const newPubKeyData = Buffer.alloc(20, '0');

    const result = await handleUpdatedScriptPayout(
      identity.getId(),
      newPubKeyData,
      blockInfo,
      identity.getPublicKeys()[0].getData(),
    );

    const newWithdrawalIdentityPublicKey = new IdentityPublicKey({
      id: 2,
      type: KeyType.ECDSA_HASH160,
      data: Buffer.from(newPubKeyData),
      readOnly: true,
      purpose: KeyPurpose.WITHDRAW,
      securityLevel: KeySecurityLevel.MASTER,
    });

    expect(identityRepositoryMock.updateRevision).to.be.calledOnceWithExactly(
      identity.getId(),
      1,
      blockInfo,
      { useTransaction: true },
    );

    expect(identityPublicKeyRepositoryMock.add).to.be.calledOnceWithExactly(
      identity.getId(),
      [newWithdrawalIdentityPublicKey],
      blockInfo,
      { useTransaction: true },
    );

    expect(result.createdEntities).to.have.lengthOf(1);
    expect(result.updatedEntities).to.have.lengthOf(1);
    expect(result.removedEntities).to.have.lengthOf(0);

    expect(result.createdEntities[0]).to.be.instanceOf(IdentityPublicKey);
    expect(result.createdEntities[0].toObject()).to.deep.equal(
      newWithdrawalIdentityPublicKey.toObject(),
    );

    expect(result.updatedEntities[0]).to.be.instanceOf(Identity);
    expect(result.updatedEntities[0].toObject()).to.deep.equal(identity.toObject());
  });

  it('should add public keys', async () => {
    const newPubKeyData = Buffer.alloc(20, '0');

    const result = await handleUpdatedScriptPayout(
      identity.getId(),
      newPubKeyData,
      blockInfo,
      new Script(),
    );

    const newWithdrawalIdentityPublicKey = new IdentityPublicKey({
      id: 2,
      type: KeyType.ECDSA_HASH160,
      data: Buffer.from(newPubKeyData),
      readOnly: true,
      purpose: KeyPurpose.WITHDRAW,
      securityLevel: KeySecurityLevel.MASTER,
    });

    expect(identityRepositoryMock.updateRevision).to.be.calledOnceWithExactly(
      identity.getId(),
      1,
      blockInfo,
      { useTransaction: true },
    );

    expect(identityPublicKeyRepositoryMock.add).to.be.calledOnceWithExactly(
      identity.getId(),
      [newWithdrawalIdentityPublicKey],
      blockInfo,
      { useTransaction: true },
    );

    expect(result.createdEntities).to.have.lengthOf(1);
    expect(result.updatedEntities).to.have.lengthOf(1);
    expect(result.removedEntities).to.have.lengthOf(0);

    expect(result.updatedEntities[0]).to.be.instanceOf(Identity);
    expect(result.updatedEntities[0].toObject()).to.deep.equal(identity.toObject());
  });
});
