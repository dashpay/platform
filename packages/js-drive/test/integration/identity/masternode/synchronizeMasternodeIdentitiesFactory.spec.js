const {
  asValue,
} = require('awilix');

const SimplifiedMNListEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListEntry');
const { hash } = require('@dashevo/dpp/lib/util/hash');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

const createTestDIContainer = require('../../../../lib/test/createTestDIContainer');
const createOperatorIdentifier = require('../../../../lib/identity/masternode/createOperatorIdentifier');

/**
 * @param {IdentityStoreRepository} identityRepository
 * @param {PublicKeyToIdentityIdStoreRepository} publicKeyToIdentityIdRepository
 * @returns {expectOperatorIdentity}
 */
function expectOperatorIdentityFactory(
  identityRepository,
  publicKeyToIdentityIdRepository,
) {
  /**
   * @typedef {expectOperatorIdentity}
   * @param {SimplifiedMNListEntry} smlEntry
   * @returns {Promise<void>}
   */
  async function expectOperatorIdentity(smlEntry) {
    // Validate operator identity

    const operatorIdentifier = createOperatorIdentifier(smlEntry);

    const operatorIdentity = await identityRepository.fetch(operatorIdentifier);

    expect(operatorIdentity)
      .to
      .exist();

    // Validate operator public keys

    const operatorPubKey = Buffer.from(smlEntry.pubKeyOperator, 'hex');

    expect(operatorIdentity.getPublicKeys())
      .to
      .have
      .lengthOf(1);

    const firstOperatorMasternodePublicKey = operatorIdentity.getPublicKeyById(0);
    expect(firstOperatorMasternodePublicKey.getType())
      .to
      .equal(IdentityPublicKey.TYPES.BLS12_381);
    expect(firstOperatorMasternodePublicKey.getData())
      .to
      .deep
      .equal(operatorPubKey);

    const firstOperatorIdentityByPublicKeyHash = await publicKeyToIdentityIdRepository
      .fetch(firstOperatorMasternodePublicKey.hash());

    expect(firstOperatorIdentityByPublicKeyHash)
      .to
      .have
      .lengthOf(1);
    expect(firstOperatorIdentityByPublicKeyHash[0].toBuffer())
      .to
      .deep
      .equal(operatorIdentifier);
  }

  return expectOperatorIdentity;
}

/**
 * @param {IdentityStoreRepository} identityRepository
 * @param {PublicKeyToIdentityIdStoreRepository} publicKeyToIdentityIdRepository
 * @returns {expectMasternodeIdentity}
 */
function expectMasternodeIdentityFactory(
  identityRepository,
  publicKeyToIdentityIdRepository,
) {
  /**
   * @typedef {expectMasternodeIdentity}
   * @param {SimplifiedMNListEntry} smlEntry
   * @param {Object} preRegTx
   * @returns {Promise<void>}
   */
  async function expectMasternodeIdentity(smlEntry, preRegTx) {
    const masternodeIdentifier = Identifier.from(
      Buffer.from(smlEntry.proRegTxHash, 'hex'),
    );

    const masternodeIdentity = await identityRepository.fetch(masternodeIdentifier);

    expect(masternodeIdentity).to.exist();

    // Validate masternode identity public keys

    expect(masternodeIdentity.getPublicKeys()).to.have.lengthOf(1);

    const masternodePublicKey = masternodeIdentity.getPublicKeyById(0);
    expect(masternodePublicKey.getType()).to.equal(IdentityPublicKey.TYPES.ECDSA_HASH160);
    expect(masternodePublicKey.getData()).to.deep.equal(
      Buffer.from(preRegTx.extraPayload.keyIDOwner, 'hex').reverse(),
    );

    const masternodeIdentityByPublicKeyHash = await publicKeyToIdentityIdRepository
      .fetch(masternodePublicKey.hash());

    expect(masternodeIdentityByPublicKeyHash).to.have.lengthOf(1);
    expect(masternodeIdentityByPublicKeyHash[0].toBuffer())
      .to.deep.equal(masternodeIdentifier);
  }

  return expectMasternodeIdentity;
}

/**
 * @param {GroveDBStore} groveDBStore
 * @returns {expectDeterministicAppHash}
 */
function expectDeterministicAppHashFactory(groveDBStore) {
  /**
   * @typedef {expectDeterministicAppHash}
   * @param {string} appHash
   * @returns {Promise<void>}
   */
  async function expectDeterministicAppHash(appHash) {
    const actualAppHash = await groveDBStore.getRootHash({ useTransaction: true });

    const actualAppHashHex = actualAppHash.toString('hex');

    expect(actualAppHashHex).to.deep.equal(appHash);
  }

  return expectDeterministicAppHash;
}

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
  let expectOperatorIdentity;
  let expectMasternodeIdentity;
  let expectDeterministicAppHash;
  let firstSyncAppHash;

  beforeEach(async function beforeEach() {
    coreHeight = 3;
    // firstSyncAppHash = '5f9981ea8bf69112408784c62ef079c19b9f57a275bf8b8df2b9b0f111cd8525';
    firstSyncAppHash = 'c7cd0e7c892ecc38695208c6ee03a3d0fc8e8de915b25eb8e2cc15b556878076';

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
        payoutAddress: 'yR843jN58m5dubmQjfUmKDDJMJzNatFV9M',
      }),
      new SimplifiedMNListEntry({
        proRegTxHash: '9673b21f45b216dce2b4ffb4a85e1471d57aed6bf8e34d961a48296fe9b7f51a',
        confirmedHash: '25e1884e4251cbf42a0f9f42666443c62d89b3bc1aae73fb1e9d753e0b2732f4',
        service: '192.168.65.2:20201',
        pubKeyOperator: '06a9789fab00deae1464ed80bda281fc833f85959b04201645e5fc25635e3e7ecda30d13d328b721af0809fca3bf3b63',
        votingAddress: 'yVRXh9Tgf9qt9tCbXmeX9FQsEYa526FMxR',
        isValid: true,
        payoutAddress: 'yR843jN58m5dubmQjfUmKDDJMJzNatFV9M',
        payoutOperatorAddress: 'yR843jN58m5dubmQjfUmKDDJMJzNatFV9M',
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

    expectOperatorIdentity = expectOperatorIdentityFactory(
      identityRepository,
      publicKeyToIdentityIdRepository,
    );

    expectMasternodeIdentity = expectMasternodeIdentityFactory(
      identityRepository,
      publicKeyToIdentityIdRepository,
    );

    expectDeterministicAppHash = expectDeterministicAppHashFactory(
      container.resolve('groveDBStore'),
    );
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should create identities for all masternodes on the first sync', async () => {
    await synchronizeMasternodeIdentities(coreHeight);

    await expectDeterministicAppHash(firstSyncAppHash);

    /**
     * Validate first masternode
     */

    // Masternode identity should be created

    await expectMasternodeIdentity(smlFixture[0], transaction1);

    // Operator identity should be created

    await expectOperatorIdentity(smlFixture[0]);

    // Masternode reward shares should be created

    const firstMasternodeIdentifier = Identifier.from(
      Buffer.from(smlFixture[0].proRegTxHash, 'hex'),
    );

    const firstOperatorIdentifier = createOperatorIdentifier(smlFixture[0]);

    let documents = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', firstMasternodeIdentifier],
          ['payToId', '==', firstOperatorIdentifier],
        ],
      },
    );

    expect(documents).to.have.lengthOf(1);

    const expectedDocumentId = Identifier.from(
      hash(
        Buffer.concat([
          firstMasternodeIdentifier,
          firstOperatorIdentifier,
        ]),
      ),
    );

    expect(documents[0].getId()).to.deep.equal(expectedDocumentId);
    expect(documents[0].getOwnerId()).to.deep.equal(firstMasternodeIdentifier);
    expect(documents[0].get('percentage')).to.equal(100);
    expect(documents[0].get('payToId')).to.deep.equal(firstOperatorIdentifier);

    /**
     * Validate second masternode
     */

    // Masternode identity should be created

    await expectMasternodeIdentity(smlFixture[1], transaction2);

    // Operator identity shouldn't be created

    const secondOperatorPubKey = Buffer.from(smlFixture[1].pubKeyOperator, 'hex');

    const secondOperatorIdentifier = Identifier.from(
      hash(
        Buffer.concat([
          Buffer.from(smlFixture[1].proRegTxHash, 'hex'),
          secondOperatorPubKey,
        ]),
      ),
    );

    const secondOperatorIdentity = await identityRepository.fetch(secondOperatorIdentifier);

    expect(secondOperatorIdentity).to.be.null();

    // Masternode reward shares shouldn't be created

    const secondMasternodeIdentifier = Identifier.from(
      Buffer.from(smlFixture[1].proRegTxHash, 'hex'),
    );

    documents = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', secondMasternodeIdentifier],
          ['payToId', '==', secondOperatorIdentifier],
        ],
      },
    );

    expect(documents).to.have.lengthOf(0);
  });

  it('should sync identities if the gap between coreHeight and lastSyncedCoreHeight > smlMaxListsLimit', async () => {
    // Sync initial list

    await synchronizeMasternodeIdentities(coreHeight);

    await expectDeterministicAppHash(firstSyncAppHash);

    // Second call

    await synchronizeMasternodeIdentities(coreHeight + 42);

    // Nothing happened

    await expectDeterministicAppHash(firstSyncAppHash);

    // Core RPC should be called

    expect(coreRpcClientMock.protx).to.have.been.calledOnceWithExactly('diff', 1, 3);
  });

  it('should create masternode identities if new masternode appeared', async () => {
    // Sync initial list

    await synchronizeMasternodeIdentities(coreHeight);

    await expectDeterministicAppHash(firstSyncAppHash);

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

    await expectDeterministicAppHash('da4c450e65eb228b58753a472c55e9a96d7f6561c0cd55ea299cf1c9ba314c05');

    // New masternode identity should be created

    await expectMasternodeIdentity(newSmlFixture[0], transaction3);

    // New operator should be created

    await expectOperatorIdentity(newSmlFixture[0]);

    // Masternode reward shares should be created

    const newMasternodeIdentifier = Identifier.from(
      Buffer.from(newSmlFixture[0].proRegTxHash, 'hex'),
    );

    const newOperatorIdentifier = createOperatorIdentifier(newSmlFixture[0]);

    const documents = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', newMasternodeIdentifier],
          ['payToId', '==', newOperatorIdentifier],
        ],
      },
    );

    expect(documents).to.have.lengthOf(1);

    const expectedDocumentId = Identifier.from(
      hash(
        Buffer.concat([
          newMasternodeIdentifier,
          newOperatorIdentifier,
        ]),
      ),
    );

    expect(documents[0].getId()).to.deep.equal(expectedDocumentId);
    expect(documents[0].getOwnerId()).to.deep.equal(newMasternodeIdentifier);
    expect(documents[0].get('percentage')).to.equal(200);
    expect(documents[0].get('payToId')).to.deep.equal(newOperatorIdentifier);
  });

  it('should remove reward shares if masternode disappeared', async () => {
    // Sync initial list

    await synchronizeMasternodeIdentities(coreHeight);

    await expectDeterministicAppHash(firstSyncAppHash);

    // Mock SML

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[1]] },
    );

    // Second call

    await synchronizeMasternodeIdentities(coreHeight + 1);

    await expectDeterministicAppHash('76579edc4260d70f3616cb22f6589f195eeeb09ac73e4253493bd1200d1511bb');

    // Masternode identity should stay

    await expectMasternodeIdentity(smlFixture[0], transaction1);

    // Operator identity should stay

    await expectOperatorIdentity(smlFixture[0]);

    // Masternode reward shares should be removed

    const removedMasternodeIdentifier = Buffer.from(smlFixture[0].proRegTxHash, 'hex');

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

    await expectDeterministicAppHash(firstSyncAppHash);

    // Mock SML

    const invalidSmlEntry = smlFixture[0].copy();
    invalidSmlEntry.isValid = false;

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[1], invalidSmlEntry] },
    );

    // Second call

    await synchronizeMasternodeIdentities(coreHeight + 1);

    await expectDeterministicAppHash('76579edc4260d70f3616cb22f6589f195eeeb09ac73e4253493bd1200d1511bb');

    const invalidMasternodeIdentifier = Identifier.from(
      Buffer.from(invalidSmlEntry.proRegTxHash, 'hex'),
    );

    // Masternode reward shares should be removed

    const documents = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', invalidMasternodeIdentifier],
        ],
      },
    );

    expect(documents).to.have.lengthOf(0);
  });

  it('should create operator identity and reward shares if PubKeyOperator was changed', async () => {
    // Initial sync

    await synchronizeMasternodeIdentities(coreHeight);

    await expectDeterministicAppHash(firstSyncAppHash);

    // Mock SML

    const changedSmlEntry = smlFixture[0].copy();
    changedSmlEntry.pubKeyOperator = '3ba9789fab00deae1464ed80bda281fc833f85959b04201645e5fc25635e3e7ecda30d13d328b721af0809fca3bf3b3b';

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[1], changedSmlEntry] },
    );

    // Second call

    await synchronizeMasternodeIdentities(coreHeight + 1);

    await expectDeterministicAppHash('efe24642da50cafb3cef3976bfd62f7c38ba3c4800b74142c4f98ff1f2ece12b');

    // Masternode identity should stay

    await expectMasternodeIdentity(smlFixture[0], transaction1);

    // Previous operator identity should stay

    await expectOperatorIdentity(smlFixture[0]);

    // New operator identity should be created

    await expectOperatorIdentity(changedSmlEntry);

    // Only new masternode reward shares should exist

    const changedMasternodeIdentifier = Identifier.from(
      Buffer.from(changedSmlEntry.proRegTxHash, 'hex'),
    );

    const documents = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', changedMasternodeIdentifier],
        ],
      },
    );

    expect(documents).to.have.lengthOf(1);

    const [document] = documents;

    const newOperatorIdentifier = createOperatorIdentifier(changedSmlEntry);

    expect(document.get('payToId')).to.deep.equal(newOperatorIdentifier);
  });
});
