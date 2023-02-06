const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const Script = require('@dashevo/dashcore-lib/lib/script');
const identitySchema = require('@dashevo/dpp/schema/identity/identity.json');
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

  beforeEach(function beforeEach() {
    identity = getIdentityFixture();

    blockInfo = new BlockInfo(1, 0, Date.now());

    getWithdrawPubKeyTypeFromPayoutScriptMock = this.sinon.stub().returns(
      IdentityPublicKey.TYPES.ECDSA_HASH160,
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
    );
  });

  it('should not update identity if identityPublicKeys max length was reached', async () => {
    const { maxItems } = identitySchema.properties.publicKeys;
    for (let i = identity.getPublicKeys().length; i < maxItems; ++i) {
      identity.publicKeys.push({
        data: 'fakePublicKey',
      });
    }

    const newPubKeyData = Buffer.alloc(20, '0');

    const result = await handleUpdatedScriptPayout(
      identity.getId(),
      newPubKeyData,
      identity.publicKeys[0].getData(),
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
      identity.publicKeys[0].getData(),
    );

    const newWithdrawalIdentityPublicKey = new IdentityPublicKey()
      .setId(2)
      .setType(IdentityPublicKey.TYPES.ECDSA_HASH160)
      .setData(Buffer.from(newPubKeyData))
      .setReadOnly(true)
      .setPurpose(IdentityPublicKey.PURPOSES.WITHDRAW)
      .setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MASTER);

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

    const newWithdrawalIdentityPublicKey = new IdentityPublicKey()
      .setId(2)
      .setType(IdentityPublicKey.TYPES.ECDSA_HASH160)
      .setData(Buffer.from(newPubKeyData))
      .setReadOnly(true)
      .setPurpose(IdentityPublicKey.PURPOSES.WITHDRAW)
      .setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MASTER);

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
