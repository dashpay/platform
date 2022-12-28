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
      update: this.sinon.stub(),
      fetch: this.sinon.stub().resolves(new StorageResult(identity, [])),
    };

    identityPublicKeyRepositoryMock = {
      store: this.sinon.stub(),
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

    expect(identityRepositoryMock.update).to.not.be.called();
    expect(identityPublicKeyRepositoryMock.store).to.not.be.called();
  });

  it('should store updated identity with updated public keys', async () => {
    const newPubKeyData = Buffer.alloc(20, '0');
    const identityPublicKeys = identity.getPublicKeys();

    const result = await handleUpdatedScriptPayout(
      identity.getId(),
      newPubKeyData,
      identity.publicKeys[0].getData(),
    );

    const identityToStore = new Identity(identity.toObject());

    identityPublicKeys[0].disabledAt = blockInfo.timeMs;

    const newWithdrawalIdentityPublicKey = new IdentityPublicKey()
      .setId(2)
      .setType(IdentityPublicKey.TYPES.ECDSA_HASH160)
      .setData(Buffer.from(newPubKeyData))
      .setPurpose(IdentityPublicKey.PURPOSES.WITHDRAW)
      .setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MASTER);

    identityPublicKeys.push(newWithdrawalIdentityPublicKey);
    identityToStore.setPublicKeys(identityPublicKeys);

    expect(identityRepositoryMock.update).to.be.calledOnceWithExactly(
      identityToStore,
      { useTransaction: true },
    );

    expect(identityPublicKeyRepositoryMock.store).to.be.calledOnceWithExactly(
      newPubKeyData,
      identity.getId(),
      { useTransaction: true },
    );

    expect(result.createdEntities).to.have.lengthOf(0);
    expect(result.updatedEntities).to.have.lengthOf(1);
    expect(result.removedEntities).to.have.lengthOf(0);

    expect(result.updatedEntities[0]).to.be.instanceOf(Identity);
    expect(result.updatedEntities[0].toJSON()).to.deep.equal(identityToStore.toJSON());
  });

  it('should store add public keys to the stored identity', async () => {
    const newPubKeyData = Buffer.alloc(20, '0');
    const identityPublicKeys = identity.getPublicKeys();

    const result = await handleUpdatedScriptPayout(
      identity.getId(),
      newPubKeyData,
      new Script(),
    );

    const identityToStore = new Identity(identity.toObject());

    const newWithdrawalIdentityPublicKey = new IdentityPublicKey()
      .setId(2)
      .setType(IdentityPublicKey.TYPES.ECDSA_HASH160)
      .setData(Buffer.from(newPubKeyData))
      .setPurpose(IdentityPublicKey.PURPOSES.WITHDRAW)
      .setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MASTER);

    identityPublicKeys.push(newWithdrawalIdentityPublicKey);
    identityToStore.setPublicKeys(identityPublicKeys);

    expect(identityRepositoryMock.update).to.be.calledOnceWithExactly(
      identityToStore,
      { useTransaction: true },
    );
    expect(identityPublicKeyRepositoryMock.store).to.be.calledOnceWithExactly(
      newPubKeyData,
      identity.getId(),
      { useTransaction: true },
    );

    expect(result.createdEntities).to.have.lengthOf(0);
    expect(result.updatedEntities).to.have.lengthOf(1);
    expect(result.removedEntities).to.have.lengthOf(0);

    expect(result.updatedEntities[0]).to.be.instanceOf(Identity);
    expect(result.updatedEntities[0].toJSON()).to.deep.equal(identityToStore.toJSON());
  });
});
