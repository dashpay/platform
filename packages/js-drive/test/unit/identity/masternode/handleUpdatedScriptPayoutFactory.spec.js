const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const Script = require('@dashevo/dashcore-lib/lib/script');
const identitySchema = require('@dashevo/dpp/schema/identity/identity.json');
const handleUpdatedScriptPayoutFactory = require('../../../../lib/identity/masternode/handleUpdatedScriptPayoutFactory');
const BlockInfo = require('../../../../lib/blockExecution/BlockInfo');

describe('handleUpdatedScriptPayoutFactory', () => {
  let handleUpdatedScriptPayout;
  let stateRepositoryMock;
  let getWithdrawPubKeyTypeFromPayoutScriptMock;
  let getPublicKeyFromPayoutScriptMock;
  let identity;
  let blockInfo;

  beforeEach(function beforeEach() {
    identity = getIdentityFixture();

    blockInfo = new BlockInfo(1, 0, Date.now());

    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchIdentity.resolves(
      identity,
    );

    getWithdrawPubKeyTypeFromPayoutScriptMock = this.sinon.stub().returns(
      IdentityPublicKey.TYPES.ECDSA_HASH160,
    );

    getPublicKeyFromPayoutScriptMock = this.sinon.stub().returns(Buffer.alloc(20, '0'));

    handleUpdatedScriptPayout = handleUpdatedScriptPayoutFactory(
      stateRepositoryMock,
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

    await handleUpdatedScriptPayout(
      identity.getId(),
      newPubKeyData,
      identity.publicKeys[0].getData(),
    );

    expect(stateRepositoryMock.updateIdentity).to.not.be.called();
    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.not.be.called();
  });

  it('should store updated identity with updated public keys', async () => {
    const newPubKeyData = Buffer.alloc(20, '0');
    const identityPublicKeys = identity.getPublicKeys();

    await handleUpdatedScriptPayout(
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

    expect(stateRepositoryMock.updateIdentity).to.be.calledOnceWithExactly(identityToStore);
    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.be.calledOnceWithExactly(
      identity.getId(),
      [newPubKeyData],
    );
  });

  it('should store add public keys to the stored identity', async () => {
    const newPubKeyData = Buffer.alloc(20, '0');
    const identityPublicKeys = identity.getPublicKeys();

    await handleUpdatedScriptPayout(
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

    expect(stateRepositoryMock.updateIdentity).to.be.calledOnceWithExactly(identityToStore);
    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.be.calledOnceWithExactly(
      identity.getId(),
      [newPubKeyData],
    );
  });
});
