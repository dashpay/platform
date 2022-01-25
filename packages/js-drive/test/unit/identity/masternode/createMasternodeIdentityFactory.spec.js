const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const createMasternodeIdentityFactory = require('../../../../lib/identity/masternode/createMasternodeIdentityFactory');

describe('createMasternodeIdentityFactory', () => {
  let createMasternodeIdentity;
  let dppMock;
  let stateRepositoryMock;
  let previousIdentityRepositoryMock;
  let previousPublicKeyToIdentityIdRepositoryMock;

  beforeEach(function beforeEach() {
    dppMock = createDPPMock(this.sinon);
    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    previousIdentityRepositoryMock = {
      store: this.sinon.stub(),
    };
    previousPublicKeyToIdentityIdRepositoryMock = {
      store: this.sinon.stub(),
    };

    createMasternodeIdentity = createMasternodeIdentityFactory(
      dppMock,
      stateRepositoryMock,
      previousIdentityRepositoryMock,
      previousPublicKeyToIdentityIdRepositoryMock,
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

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.have.been.calledOnceWithExactly(
      identity.getId(),
      publicKeyHashes,
    );

    expect(previousIdentityRepositoryMock.store).to.be.not.called();
    expect(previousPublicKeyToIdentityIdRepositoryMock.store).to.be.not.called();
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

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

    expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.have.been.calledOnceWithExactly(
      identity.getId(),
      publicKeyHashes,
    );

    expect(previousIdentityRepositoryMock.store).to.have.been.calledOnceWithExactly(identity);
    expect(previousPublicKeyToIdentityIdRepositoryMock.store).to.have.been.calledOnceWithExactly(
      publicKeyHashes[0],
      identity.getId(),
    );
  });
});
