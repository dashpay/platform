const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const identitySchema = require('@dashevo/dpp/schema/identity/identity.json');
const handleUpdatedScriptPayoutFactory = require('../../../../lib/identity/masternode/handleUpdatedScriptPayoutFactory');

describe('handleUpdatedScriptPayoutFactory', () => {
  let handleUpdatedScriptPayout;
  let stateRepositoryMock;
  let getWithdrawPubKeyTypeFromPayoutScriptMock;
  let identity;
  let fakeTimeDate;
  let fakeTime;

  beforeEach(function beforeEach() {
    identity = getIdentityFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchIdentity.resolves(
      identity,
    );

    getWithdrawPubKeyTypeFromPayoutScriptMock = this.sinon.stub().returns(
      IdentityPublicKey.TYPES.ECDSA_HASH160,
    );

    handleUpdatedScriptPayout = handleUpdatedScriptPayoutFactory(
      stateRepositoryMock,
      getWithdrawPubKeyTypeFromPayoutScriptMock,
    );

    fakeTimeDate = new Date();
    fakeTime = this.sinon.useFakeTimers(fakeTimeDate);
  });

  afterEach(() => {
    fakeTime.reset();
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

    expect(stateRepositoryMock.storeIdentity).to.not.be.called();
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

    identityPublicKeys[0].disabledAt = fakeTimeDate.getTime();

    const newWithdrawalIdentityPublicKey = new IdentityPublicKey()
      .setId(2)
      .setType(IdentityPublicKey.TYPES.ECDSA_HASH160)
      .setData(Buffer.from(newPubKeyData))
      .setPurpose(IdentityPublicKey.PURPOSES.WITHDRAW)
      .setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MASTER);

    identityPublicKeys.push(newWithdrawalIdentityPublicKey);
    identityToStore.setPublicKeys(identityPublicKeys);

    expect(stateRepositoryMock.storeIdentity).to.be.calledOnceWithExactly(identityToStore);
    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.be.calledOnceWithExactly(
      identity.getId(),
      [newPubKeyData],
    );
  });
});
