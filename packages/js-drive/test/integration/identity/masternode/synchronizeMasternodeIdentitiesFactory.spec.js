const {
  asValue,
} = require('awilix');

const SimplifiedMNListEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListEntry');
const { hash } = require('@dashevo/dpp/lib/util/hash');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

const createTestDIContainer = require('../../../../lib/test/createTestDIContainer');

describe('synchronizeMasternodeIdentitiesFactory', () => {
  let container;
  let coreHeight;
  let rawDiff;
  let fetchTransactionMock;
  let smlStoreMock;
  let smlFixture;
  let transaction1;
  let transaction2;
  let synchronizeMasternodeIdentities;
  let rewardsDataContract;
  let identityRepository;
  let documentRepository;
  let publicKeyToIdentityIdRepository;
  let coreRpcClientMock;

  beforeEach(async function beforeEach() {
    coreHeight = 3;

    container = await createTestDIContainer();

    // Mock fetchTransaction

    fetchTransactionMock = this.sinon.stub();

    transaction1 = {
      extraPayload: {
        operatorReward: 100,
        keyIDOwner: Buffer.alloc(20).fill('a').toString('hex'),
      },
    };

    transaction2 = {
      extraPayload: {
        operatorReward: 0,
        keyIDOwner: Buffer.alloc(20).fill('b').toString('hex'),
      },
    };

    fetchTransactionMock.withArgs('954112bb018895896cfa3c3d00761a045fc16b22f2170c1fbb029a2936c68f16').resolves(transaction1);
    fetchTransactionMock.withArgs('9673b21f45b216dce2b4ffb4a85e1471d57aed6bf8e34d961a48296fe9b7f51a').resolves(transaction2);

    container.register('fetchTransaction', asValue(fetchTransactionMock));

    // Mock Core RPC

    coreRpcClientMock = {
      protx: this.sinon.stub().resolves({
        result: rawDiff,
      }),
    };

    container.register('coreRpcClient', asValue(coreRpcClientMock));

    // Mock SML
    smlFixture = [
      new SimplifiedMNListEntry({
        proRegTxHash: '954112bb018895896cfa3c3d00761a045fc16b22f2170c1fbb029a2936c68f16',
        confirmedHash: '1de71625dbc973e2377ebd7da4fe6f8a8eb8af8c5a99373e36151a4fbe9947cc',
        service: '192.168.65.2:20101',
        pubKeyOperator: '8e4c8c144bd6c62640fe3ae295973d512f83f7f541525a5da3c91e77ec02ff4dcd214e7431b7d2cc28e420ebfeb612ee',
        votingAddress: 'yfLLjdEynGQBdoPcCDUNAxu6pksYGzXKA4',
        isValid: true,
      }),
      new SimplifiedMNListEntry({
        proRegTxHash: '9673b21f45b216dce2b4ffb4a85e1471d57aed6bf8e34d961a48296fe9b7f51a',
        confirmedHash: '25e1884e4251cbf42a0f9f42666443c62d89b3bc1aae73fb1e9d753e0b2732f4',
        service: '192.168.65.2:20201',
        pubKeyOperator: '06a9789fab00deae1464ed80bda281fc833f85959b04201645e5fc25635e3e7ecda30d13d328b721af0809fca3bf3b63',
        votingAddress: 'yVRXh9Tgf9qt9tCbXmeX9FQsEYa526FMxR',
        isValid: true,
      }),
    ];

    smlStoreMock = {
      getSMLbyHeight: this.sinon.stub().returns({ mnList: smlFixture }),
    };

    const simplifiedMasternodeListMock = {
      getStore: this.sinon.stub().returns(smlStoreMock),
    };

    container.register('simplifiedMasternodeList', asValue(simplifiedMasternodeListMock));

    const createInitialStateStructure = container.resolve('createInitialStateStructure');
    await createInitialStateStructure();

    const registerSystemDataContract = container.resolve('registerSystemDataContract');
    const masternodeRewardSharesContractId = container.resolve('masternodeRewardSharesContractId');
    const masternodeRewardSharesOwnerId = container.resolve('masternodeRewardSharesOwnerId');
    const masternodeRewardSharesOwnerPublicKey = container.resolve('masternodeRewardSharesOwnerPublicKey');
    const masternodeRewardSharesDocuments = container.resolve('masternodeRewardSharesDocuments');

    rewardsDataContract = await registerSystemDataContract(
      masternodeRewardSharesOwnerId,
      masternodeRewardSharesContractId,
      masternodeRewardSharesOwnerPublicKey,
      masternodeRewardSharesDocuments,
    );

    synchronizeMasternodeIdentities = container.resolve('synchronizeMasternodeIdentities');

    identityRepository = container.resolve('identityRepository');
    documentRepository = container.resolve('documentRepository');
    publicKeyToIdentityIdRepository = container.resolve('publicKeyToIdentityIdRepository');
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should create identities for all masternodes on the first sync', async () => {
    await synchronizeMasternodeIdentities(coreHeight);

    /**
     * Validate first masternode
     */

    // Validate masternode identity

    const firstMasternodeIdentifier = hash(
      Buffer.from(smlFixture[0].proRegTxHash, 'hex'),
    );

    const firstMasternodeIdentity = await identityRepository.fetch(
      Identifier.from(firstMasternodeIdentifier),
    );

    expect(firstMasternodeIdentity).to.exist();

    // Validate masternode public keys

    expect(firstMasternodeIdentity.getPublicKeys()).to.have.lengthOf(1);

    const firstMasternodePublicKey = firstMasternodeIdentity.getPublicKeyById(0);
    expect(firstMasternodePublicKey.getType()).to.equal(IdentityPublicKey.TYPES.ECDSA_HASH160);
    expect(firstMasternodePublicKey.getData()).to.deep.equal(Buffer.from(transaction1.extraPayload.keyIDOwner, 'hex'));

    const firstMasternodeIdentityByPublicKeyHash = await publicKeyToIdentityIdRepository
      .fetch(firstMasternodePublicKey.hash());

    expect(firstMasternodeIdentityByPublicKeyHash).to.have.lengthOf(1);
    expect(firstMasternodeIdentityByPublicKeyHash[0].toBuffer())
      .to.deep.equal(firstMasternodeIdentifier);

    // Validate operator identity

    const firstOperatorPubKey = Buffer.from(smlFixture[0].pubKeyOperator, 'hex');

    const firstOperatorIdentityId = hash(
      Buffer.concat([
        Buffer.from(smlFixture[0].proRegTxHash, 'hex'),
        firstOperatorPubKey,
      ]),
    );

    const firstOperatorIdentity = await identityRepository.fetch(
      Identifier.from(firstOperatorIdentityId),
    );

    expect(firstOperatorIdentity).to.exist();

    // Validate operator public keys

    expect(firstOperatorIdentity.getPublicKeys()).to.have.lengthOf(1);

    const firstOperatorMasternodePublicKey = firstOperatorIdentity.getPublicKeyById(0);
    expect(firstOperatorMasternodePublicKey.getType()).to.equal(IdentityPublicKey.TYPES.BLS12_381);
    expect(firstOperatorMasternodePublicKey.getData()).to.deep.equal(firstOperatorPubKey);

    const firstOperatorIdentityByPublicKeyHash = await publicKeyToIdentityIdRepository
      .fetch(firstOperatorMasternodePublicKey.hash());

    expect(firstOperatorIdentityByPublicKeyHash).to.have.lengthOf(1);
    expect(firstOperatorIdentityByPublicKeyHash[0].toBuffer())
      .to.deep.equal(firstOperatorIdentityId);

    // Validate masternode reward shares

    let documents = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', firstMasternodeIdentifier],
          ['payToId', '==', firstOperatorIdentityId],
        ],
      },
    );

    expect(documents).to.have.lengthOf(1);

    const expectedDocumentId = Identifier.from(
      hash(
        Buffer.concat([
          firstMasternodeIdentifier,
          firstOperatorIdentityId,
        ]),
      ),
    );

    expect(documents[0].getId()).to.deep.equal(expectedDocumentId);
    expect(documents[0].getOwnerId()).to.deep.equal(firstMasternodeIdentifier);
    expect(documents[0].get('percentage')).to.equal(100);
    expect(documents[0].get('payToId')).to.deep.equal(firstOperatorIdentityId);

    /**
     * Validate second masternode
     */

    // Validate masternode identity

    const secondMasternodeIdentifier = hash(
      Buffer.from(smlFixture[1].proRegTxHash, 'hex'),
    );

    const secondMasternodeIdentity = await identityRepository.fetch(
      Identifier.from(secondMasternodeIdentifier),
    );

    expect(secondMasternodeIdentity).to.exist();

    // Validate masternode identity public keys

    expect(secondMasternodeIdentity.getPublicKeys()).to.have.lengthOf(1);

    const secondMasternodePublicKey = secondMasternodeIdentity.getPublicKeyById(0);
    expect(secondMasternodePublicKey.getType()).to.equal(IdentityPublicKey.TYPES.ECDSA_HASH160);
    expect(secondMasternodePublicKey.getData()).to.deep.equal(Buffer.from(transaction2.extraPayload.keyIDOwner, 'hex'));

    const secondMasternodeIdentityByPublicKeyHash = await publicKeyToIdentityIdRepository
      .fetch(secondMasternodePublicKey.hash());

    expect(secondMasternodeIdentityByPublicKeyHash).to.have.lengthOf(1);
    expect(secondMasternodeIdentityByPublicKeyHash[0].toBuffer())
      .to.deep.equal(secondMasternodeIdentifier);

    // Validate operator identity (shouldn't be created)

    const secondOperatorPubKey = Buffer.from(smlFixture[1].pubKeyOperator, 'hex');

    const secondOperatorIdentityId = hash(
      Buffer.concat([
        Buffer.from(smlFixture[1].proRegTxHash, 'hex'),
        secondOperatorPubKey,
      ]),
    );

    const secondOperatorIdentity = await identityRepository.fetch(
      Identifier.from(secondOperatorIdentityId),
    );

    expect(secondOperatorIdentity).to.be.null();

    // Validate masternode reward shares (shouldn't be created)

    documents = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', secondMasternodeIdentifier],
          ['payToId', '==', secondOperatorIdentityId],
        ],
      },
    );

    expect(documents).to.have.lengthOf(0);
  });

  it('should sync identities if the gap between coreHeight and lastSyncedCoreHeight > smlMaxListsLimit', async () => {
    // Sync initial list

    await synchronizeMasternodeIdentities(coreHeight);

    // Second call

    await synchronizeMasternodeIdentities(coreHeight + 42);

    expect(coreRpcClientMock.protx).to.have.been.calledOnceWithExactly('diff', 1, 3);
  });

  it('should create masternode identities if new masternode appeared', async () => {
    // Sync initial list

    await synchronizeMasternodeIdentities(coreHeight);

    // Mock SML

    const newSmlFixture = [
      new SimplifiedMNListEntry({
        proRegTxHash: '3b73b21f45b216dce2b4ffb4a85e1471d57aed6bf8e34d961a48296fe9b7f53b',
        confirmedHash: '3be1884e4251cbf42a0f9f42666443c62d89b3bc1aae73fb1e9d753e0b27323b',
        service: '192.168.65.3:20201',
        pubKeyOperator: '3ba9789fab00deae1464ed80bda281fc833f85959b04201645e5fc25635e3e7ecda30d13d328b721af0809fca3bf3b3b',
        votingAddress: 'yVey9g4fsN3RY3ZjQ7HqiKEH2zEVAG95EN',
        isValid: true,
      }),
    ];

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: smlFixture.concat(newSmlFixture) },
    );

    // Mock fetchTransaction

    const transaction3 = {
      extraPayload: {
        operatorReward: 200,
        keyIDOwner: Buffer.alloc(20).fill('c').toString('hex'),
      },
    };

    fetchTransactionMock.withArgs('3b73b21f45b216dce2b4ffb4a85e1471d57aed6bf8e34d961a48296fe9b7f53b').resolves(transaction3);

    // Second call

    await synchronizeMasternodeIdentities(coreHeight + 1);

    const newMasternodeIdentifier = hash(
      Buffer.from(newSmlFixture[0].proRegTxHash, 'hex'),
    );
    const newMasternodeIdentity = await identityRepository.fetch(
      Identifier.from(newMasternodeIdentifier),
    );

    expect(newMasternodeIdentity).to.exist();

    expect(newMasternodeIdentity.getPublicKeys()).to.have.lengthOf(1);

    const newMasternodePublicKey = newMasternodeIdentity.getPublicKeyById(0);
    expect(newMasternodePublicKey.getType()).to.equal(IdentityPublicKey.TYPES.ECDSA_HASH160);
    expect(newMasternodePublicKey.getData()).to.deep.equal(Buffer.from(transaction3.extraPayload.keyIDOwner, 'hex'));

    const newMasternodeIdentityByPublicKeyHash = await publicKeyToIdentityIdRepository
      .fetch(newMasternodePublicKey.hash());

    expect(newMasternodeIdentityByPublicKeyHash).to.have.lengthOf(1);
    expect(newMasternodeIdentityByPublicKeyHash[0].toBuffer())
      .to.deep.equal(newMasternodeIdentifier);

    // Validate operator identity

    const newOperatorPubKey = Buffer.from(newSmlFixture[0].pubKeyOperator, 'hex');

    const newOperatorIdentityId = hash(
      Buffer.concat([
        Buffer.from(newSmlFixture[0].proRegTxHash, 'hex'),
        newOperatorPubKey,
      ]),
    );

    const firstOperatorIdentity = await identityRepository.fetch(
      Identifier.from(newOperatorIdentityId),
    );

    expect(firstOperatorIdentity).to.exist();

    // Validate operator public keys

    expect(firstOperatorIdentity.getPublicKeys()).to.have.lengthOf(1);

    const firstOperatorMasternodePublicKey = firstOperatorIdentity.getPublicKeyById(0);
    expect(firstOperatorMasternodePublicKey.getType()).to.equal(IdentityPublicKey.TYPES.BLS12_381);
    expect(firstOperatorMasternodePublicKey.getData()).to.deep.equal(newOperatorPubKey);

    const firstOperatorIdentityByPublicKeyHash = await publicKeyToIdentityIdRepository
      .fetch(firstOperatorMasternodePublicKey.hash());

    expect(firstOperatorIdentityByPublicKeyHash).to.have.lengthOf(1);
    expect(firstOperatorIdentityByPublicKeyHash[0].toBuffer())
      .to.deep.equal(newOperatorIdentityId);

    // Validate masternode reward shares

    const documents = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', newMasternodeIdentifier],
          ['payToId', '==', newOperatorIdentityId],
        ],
      },
    );

    expect(documents).to.have.lengthOf(1);

    const expectedDocumentId = Identifier.from(
      hash(
        Buffer.concat([
          newMasternodeIdentifier,
          newOperatorIdentityId,
        ]),
      ),
    );

    expect(documents[0].getId()).to.deep.equal(expectedDocumentId);
    expect(documents[0].getOwnerId()).to.deep.equal(newMasternodeIdentifier);
    expect(documents[0].get('percentage')).to.equal(200);
    expect(documents[0].get('payToId')).to.deep.equal(newOperatorIdentityId);
  });

  it('should remove reward shares if masternode disappeared', async () => {
    // Sync initial list

    await synchronizeMasternodeIdentities(coreHeight);

    // Mock SML

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[1]] },
    );

    // Second call

    await synchronizeMasternodeIdentities(coreHeight + 1);

    const removedMasternodeIdentifier = hash(
      Buffer.from(smlFixture[0].proRegTxHash, 'hex'),
    );

    const removedMasternodeIdentity = await identityRepository.fetch(
      Identifier.from(removedMasternodeIdentifier),
    );

    expect(removedMasternodeIdentity).to.exist();

    // Validate masternode identity public keys

    expect(removedMasternodeIdentity.getPublicKeys()).to.have.lengthOf(1);

    const removedMasternodePublicKey = removedMasternodeIdentity.getPublicKeyById(0);
    expect(removedMasternodePublicKey.getType()).to.equal(IdentityPublicKey.TYPES.ECDSA_HASH160);
    expect(removedMasternodePublicKey.getData()).to.deep.equal(Buffer.from(transaction1.extraPayload.keyIDOwner, 'hex'));

    const removedMasternodeIdentityByPublicKeyHash = await publicKeyToIdentityIdRepository
      .fetch(removedMasternodePublicKey.hash());

    expect(removedMasternodeIdentityByPublicKeyHash).to.have.lengthOf(1);
    expect(removedMasternodeIdentityByPublicKeyHash[0].toBuffer())
      .to.deep.equal(removedMasternodeIdentifier);

    // Validate operator identity (shouldn't be created)

    const operatorPubKey = Buffer.from(smlFixture[0].pubKeyOperator, 'hex');

    const operatorIdentityId = hash(
      Buffer.concat([
        Buffer.from(smlFixture[0].proRegTxHash, 'hex'),
        operatorPubKey,
      ]),
    );

    const operatorIdentity = await identityRepository.fetch(
      Identifier.from(operatorIdentityId),
    );

    expect(operatorIdentity).to.exist();

    // Validate operator public keys

    expect(operatorIdentity.getPublicKeys()).to.have.lengthOf(1);

    const firstOperatorMasternodePublicKey = operatorIdentity.getPublicKeyById(0);
    expect(firstOperatorMasternodePublicKey.getType()).to.equal(IdentityPublicKey.TYPES.BLS12_381);
    expect(firstOperatorMasternodePublicKey.getData()).to.deep.equal(operatorPubKey);

    const firstOperatorIdentityByPublicKeyHash = await publicKeyToIdentityIdRepository
      .fetch(firstOperatorMasternodePublicKey.hash());

    expect(firstOperatorIdentityByPublicKeyHash).to.have.lengthOf(1);
    expect(firstOperatorIdentityByPublicKeyHash[0].toBuffer())
      .to.deep.equal(operatorIdentityId);

    // Validate masternode reward shares (shouldn't be created)

    const documents = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', removedMasternodeIdentifier],
        ],
      },
    );

    expect(documents).to.have.lengthOf(0);
  });

  it('should remove reward shares if masternode is not valid', async () => {
    // Sync initial list

    await synchronizeMasternodeIdentities(coreHeight);

    // Mock SML

    const invalidSmlEntry = smlFixture[0].copy();
    invalidSmlEntry.isValid = false;

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[1], invalidSmlEntry] },
    );

    // Second call

    await synchronizeMasternodeIdentities(coreHeight + 1);

    const invalidMasternodeIdentifier = hash(
      Buffer.from(invalidSmlEntry.proRegTxHash, 'hex'),
    );

    const invalidMasternodeId = Identifier.from(invalidMasternodeIdentifier);

    // Validate masternode reward shares

    let documents = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', invalidMasternodeId],
        ],
      },
    );

    expect(documents).to.have.lengthOf(0);
  });

  it('should update create operator identity and reward shares if PubKeyOperator was changed', async () => {
    // Initial sync

    await synchronizeMasternodeIdentities(coreHeight);

    // Mock SML

    const changedSmlEntry = smlFixture[1].copy();
    changedSmlEntry.pubKeyOperator = '3ba9789fab00deae1464ed80bda281fc833f85959b04201645e5fc25635e3e7ecda30d13d328b721af0809fca3bf3b3b';

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[0], changedSmlEntry] },
    );

    // smlFixture[1].pubKeyOperator = 'cca9789fab00deae1464ed80bda281fc833f85959b04201645e5fc25635e3e7ecda30d13d328b721af0809fca3bf3bcc'

    await synchronizeMasternodeIdentities(coreHeight + 1);

    const removedIdentifier = hash(
      Buffer.from(smlFixture[0].proRegTxHash, 'hex'),
    );
    const removedIdentity = await transactionalStateRepository.fetchIdentity(removedIdentifier);

    expect(removedIdentity).to.be.null();
  });
});
