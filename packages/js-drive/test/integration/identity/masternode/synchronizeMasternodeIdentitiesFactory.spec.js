const {
  asValue,
} = require('awilix');

const SimplifiedMNListEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListEntry');
const { hash } = require('@dashevo/dpp/lib/util/hash');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

const Address = require('@dashevo/dashcore-lib/lib/address');
const Script = require('@dashevo/dashcore-lib/lib/script');
const createTestDIContainer = require('../../../../lib/test/createTestDIContainer');
const createOperatorIdentifier = require('../../../../lib/identity/masternode/createOperatorIdentifier');

/**
 * @param {IdentityStoreRepository} identityRepository
 * @param {PublicKeyToIdentityIdStoreRepository} publicKeyToIdentityIdRepository
 * @param {getWithdrawPubKeyTypeFromPayoutScript} getWithdrawPubKeyTypeFromPayoutScript
 * @param {getPublicKeyFromPayoutScript} getPublicKeyFromPayoutScript
 * @returns {expectOperatorIdentity}
 */
function expectOperatorIdentityFactory(
  identityRepository,
  publicKeyToIdentityIdRepository,
  getWithdrawPubKeyTypeFromPayoutScript,
  getPublicKeyFromPayoutScript,
) {
  /**
   * @typedef {expectOperatorIdentity}
   * @param {SimplifiedMNListEntry} smlEntry
   * @param {Address} [previousPayoutAddress]
   * @param {Address} [payoutAddress]
   * @returns {Promise<void>}
   */
  async function expectOperatorIdentity(
    smlEntry,
    previousPayoutAddress,
    payoutAddress,
  ) {
    // Validate operator identity

    const operatorIdentifier = createOperatorIdentifier(smlEntry);

    const operatorIdentityResult = await identityRepository.fetch(operatorIdentifier);

    const operatorIdentity = operatorIdentityResult.getValue();

    expect(operatorIdentity)
      .to
      .exist();

    // Validate operator public keys

    const operatorPubKey = Buffer.from(smlEntry.pubKeyOperator, 'hex');

    let publicKeysNum = 1;
    if (payoutAddress) {
      publicKeysNum += 1;
    }
    if (previousPayoutAddress) {
      publicKeysNum += 1;
    }

    expect(operatorIdentity.getPublicKeys())
      .to
      .have
      .lengthOf(publicKeysNum);

    const firstOperatorMasternodePublicKey = operatorIdentity.getPublicKeyById(0);
    expect(firstOperatorMasternodePublicKey.getType())
      .to
      .equal(IdentityPublicKey.TYPES.BLS12_381);
    expect(firstOperatorMasternodePublicKey.getData())
      .to
      .deep
      .equal(operatorPubKey);

    const firstOperatorIdentityByPublicKeyHashResult = await publicKeyToIdentityIdRepository
      .fetch(firstOperatorMasternodePublicKey.hash());

    const firstOperatorIdentityByPublicKeyHash = firstOperatorIdentityByPublicKeyHashResult
      .getValue();

    expect(firstOperatorIdentityByPublicKeyHash)
      .to
      .have
      .lengthOf(1);
    expect(firstOperatorIdentityByPublicKeyHash[0].toBuffer())
      .to
      .deep
      .equal(operatorIdentifier);

    let i = 0;

    if (previousPayoutAddress) {
      i += 1;
      const payoutScript = new Script(previousPayoutAddress);
      const publicKeyType = getWithdrawPubKeyTypeFromPayoutScript(payoutScript);

      const payoutPublicKey = operatorIdentity.getPublicKeyById(i);
      expect(payoutPublicKey.getType()).to.equal(publicKeyType);
      expect(payoutPublicKey.getData()).to.deep.equal(
        getPublicKeyFromPayoutScript(payoutScript, publicKeyType),
      );

      const masternodeIdentityByPayoutPublicKeyHashResult = await publicKeyToIdentityIdRepository
        .fetch(payoutPublicKey.hash());

      const masternodeIdentityByPayoutPublicKeyHash = masternodeIdentityByPayoutPublicKeyHashResult
        .getValue();

      expect(masternodeIdentityByPayoutPublicKeyHash).to.have.lengthOf(1);
      expect(masternodeIdentityByPayoutPublicKeyHash[0].toBuffer())
        .to.deep.equal(operatorIdentifier);
    }

    if (payoutAddress) {
      i += 1;
      const payoutScript = new Script(payoutAddress);
      const publicKeyType = getWithdrawPubKeyTypeFromPayoutScript(payoutScript);

      const payoutPublicKey = operatorIdentity.getPublicKeyById(i);
      expect(payoutPublicKey.getType()).to.equal(publicKeyType);
      expect(payoutPublicKey.getData()).to.deep.equal(
        getPublicKeyFromPayoutScript(payoutScript, publicKeyType),
      );

      const masternodeIdentityByPayoutPublicKeyHashResult = await publicKeyToIdentityIdRepository
        .fetch(payoutPublicKey.hash());

      const masternodeIdentityByPayoutPublicKeyHash = masternodeIdentityByPayoutPublicKeyHashResult
        .getValue();

      expect(masternodeIdentityByPayoutPublicKeyHash).to.have.lengthOf(1);
      expect(masternodeIdentityByPayoutPublicKeyHash[0].toBuffer())
        .to.deep.equal(operatorIdentifier);
    }
  }

  return expectOperatorIdentity;
}

/**
 * @param {IdentityStoreRepository} identityRepository
 * @param {PublicKeyToIdentityIdStoreRepository} publicKeyToIdentityIdRepository
 * @param {getWithdrawPubKeyTypeFromPayoutScript} getWithdrawPubKeyTypeFromPayoutScript
 * @param {getPublicKeyFromPayoutScript} getPublicKeyFromPayoutScript
 * @returns {expectMasternodeIdentity}
 */
function expectMasternodeIdentityFactory(
  identityRepository,
  publicKeyToIdentityIdRepository,
  getWithdrawPubKeyTypeFromPayoutScript,
  getPublicKeyFromPayoutScript,
) {
  /**
   * @typedef {expectMasternodeIdentity}
   * @param {SimplifiedMNListEntry} smlEntry
   * @param {Object} proRegTx
   * @param {Address} [previousPayoutAddress]
   * @param {Address} [payoutAddress]
   * @returns {Promise<void>}
   */
  async function expectMasternodeIdentity(
    smlEntry,
    proRegTx,
    previousPayoutAddress,
    payoutAddress,
  ) {
    const masternodeIdentifier = Identifier.from(
      Buffer.from(smlEntry.proRegTxHash, 'hex'),
    );

    const masternodeIdentityResult = await identityRepository.fetch(masternodeIdentifier);

    const masternodeIdentity = masternodeIdentityResult.getValue();

    expect(masternodeIdentity).to.be.not.null();

    // Validate masternode identity public keys
    let publicKeysNum = 1;
    if (payoutAddress) {
      publicKeysNum += 1;
    }
    if (previousPayoutAddress) {
      publicKeysNum += 1;
    }

    expect(masternodeIdentity.getPublicKeys()).to.have.lengthOf(publicKeysNum);

    const masternodePublicKey = masternodeIdentity.getPublicKeyById(0);
    expect(masternodePublicKey.getType()).to.equal(IdentityPublicKey.TYPES.ECDSA_HASH160);
    expect(masternodePublicKey.getData()).to.deep.equal(
      Buffer.from(proRegTx.extraPayload.keyIDOwner, 'hex').reverse(),
    );

    const masternodeIdentityByPublicKeyHashResult = await publicKeyToIdentityIdRepository
      .fetch(masternodePublicKey.hash());

    const masternodeIdentityByPublicKeyHash = masternodeIdentityByPublicKeyHashResult.getValue();

    expect(masternodeIdentityByPublicKeyHash).to.have.lengthOf(1);
    expect(masternodeIdentityByPublicKeyHash[0].toBuffer())
      .to.deep.equal(masternodeIdentifier);

    let i = 0;

    if (previousPayoutAddress) {
      i += 1;
      const payoutScript = new Script(previousPayoutAddress);
      const publicKeyType = getWithdrawPubKeyTypeFromPayoutScript(payoutScript);

      const payoutPublicKey = masternodeIdentity.getPublicKeyById(i);
      expect(payoutPublicKey.getType()).to.equal(publicKeyType);
      expect(payoutPublicKey.getData()).to.deep.equal(
        getPublicKeyFromPayoutScript(payoutScript, publicKeyType),
      );

      const masternodeIdentityByPayoutPublicKeyHashResult = await publicKeyToIdentityIdRepository
        .fetch(payoutPublicKey.hash());

      const masternodeIdentityByPayoutPublicKeyHash = masternodeIdentityByPayoutPublicKeyHashResult
        .getValue();

      expect(masternodeIdentityByPayoutPublicKeyHash).to.have.lengthOf(1);
      expect(masternodeIdentityByPayoutPublicKeyHash[0].toBuffer())
        .to.deep.equal(masternodeIdentifier);
    }

    if (payoutAddress) {
      i += 1;
      const payoutScript = new Script(payoutAddress);
      const publicKeyType = getWithdrawPubKeyTypeFromPayoutScript(payoutScript);

      const payoutPublicKey = masternodeIdentity.getPublicKeyById(i);
      expect(payoutPublicKey.getType()).to.equal(publicKeyType);
      expect(payoutPublicKey.getData()).to.deep.equal(
        getPublicKeyFromPayoutScript(payoutScript, publicKeyType),
      );

      const masternodeIdentityByPayoutPublicKeyHashResult = await publicKeyToIdentityIdRepository
        .fetch(payoutPublicKey.hash());

      const masternodeIdentityByPayoutPublicKeyHash = masternodeIdentityByPayoutPublicKeyHashResult
        .getValue();

      expect(masternodeIdentityByPayoutPublicKeyHash).to.have.lengthOf(1);
      expect(masternodeIdentityByPayoutPublicKeyHash[0].toBuffer())
        .to.deep.equal(masternodeIdentifier);
    }
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
    firstSyncAppHash = '401bd11b196fc470275c14d28d2e76f764434c92af4af09ebad040a8fb51ca74';

    container = await createTestDIContainer();

    const blockExecutionContext = container.resolve('blockExecutionContext');
    blockExecutionContext.getHeader = this.sinon.stub().returns(
      { time: { seconds: 1651585250 } },
    );

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
        payoutOperatorAddress: 'yNjsnYM16J5NZPA2P8BKJG3MKfUD7XHAFE',
      }),
      new SimplifiedMNListEntry({
        proRegTxHash: '9673b21f45b216dce2b4ffb4a85e1471d57aed6bf8e34d961a48296fe9b7f51a',
        confirmedHash: '25e1884e4251cbf42a0f9f42666443c62d89b3bc1aae73fb1e9d753e0b2732f4',
        service: '192.168.65.2:20201',
        pubKeyOperator: '06a9789fab00deae1464ed80bda281fc833f85959b04201645e5fc25635e3e7ecda30d13d328b721af0809fca3bf3b63',
        votingAddress: 'yVRXh9Tgf9qt9tCbXmeX9FQsEYa526FMxR',
        isValid: true,
        payoutAddress: 'ycL7L4mhYoaZdm9TH85svvpfeKtdfo249u',
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
    const masternodeRewardSharesOwnerMasterPublicKey = container.resolve('masternodeRewardSharesOwnerMasterPublicKey');
    const masternodeRewardSharesOwnerSecondPublicKey = container.resolve('masternodeRewardSharesOwnerSecondPublicKey');
    const masternodeRewardSharesDocuments = container.resolve('masternodeRewardSharesDocuments');

    rewardsDataContract = await registerSystemDataContract(
      masternodeRewardSharesOwnerId,
      masternodeRewardSharesContractId,
      masternodeRewardSharesOwnerMasterPublicKey,
      masternodeRewardSharesOwnerSecondPublicKey,
      masternodeRewardSharesDocuments,
    );

    synchronizeMasternodeIdentities = container.resolve('synchronizeMasternodeIdentities');

    identityRepository = container.resolve('identityRepository');
    documentRepository = container.resolve('documentRepository');
    publicKeyToIdentityIdRepository = container.resolve('publicKeyToIdentityIdRepository');
    const getWithdrawPubKeyTypeFromPayoutScript = container.resolve('getWithdrawPubKeyTypeFromPayoutScript');
    const getPublicKeyFromPayoutScript = container.resolve('getPublicKeyFromPayoutScript');

    expectOperatorIdentity = expectOperatorIdentityFactory(
      identityRepository,
      publicKeyToIdentityIdRepository,
      getWithdrawPubKeyTypeFromPayoutScript,
      getPublicKeyFromPayoutScript,
    );

    expectMasternodeIdentity = expectMasternodeIdentityFactory(
      identityRepository,
      publicKeyToIdentityIdRepository,
      getWithdrawPubKeyTypeFromPayoutScript,
      getPublicKeyFromPayoutScript,
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

    await expectMasternodeIdentity(
      smlFixture[0],
      transaction1,
      Address.fromString(smlFixture[0].payoutAddress),
    );

    // Operator identity should be created

    await expectOperatorIdentity(smlFixture[0]);

    // Masternode reward shares should be created

    const firstMasternodeIdentifier = Identifier.from(
      Buffer.from(smlFixture[0].proRegTxHash, 'hex'),
    );

    const firstOperatorIdentifier = createOperatorIdentifier(smlFixture[0]);

    let documentsResult = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', firstMasternodeIdentifier],
          ['payToId', '==', firstOperatorIdentifier],
        ],
      },
    );

    let documents = documentsResult.getValue();

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

    await expectMasternodeIdentity(
      smlFixture[1],
      transaction2,
      undefined,
      Address.fromString(smlFixture[1].payoutAddress),
    );

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

    const secondOperatorIdentityResult = await identityRepository.fetch(secondOperatorIdentifier);

    const secondOperatorIdentity = secondOperatorIdentityResult.getValue();

    expect(secondOperatorIdentity).to.be.null();

    // Masternode reward shares shouldn't be created

    const secondMasternodeIdentifier = Identifier.from(
      Buffer.from(smlFixture[1].proRegTxHash, 'hex'),
    );

    documentsResult = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', secondMasternodeIdentifier],
          ['payToId', '==', secondOperatorIdentifier],
        ],
      },
    );

    documents = documentsResult.getValue();

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
        payoutAddress: '7UkJidhNjEPJCQnCTXeaJKbJmL4JuyV66w',
        payoutOperatorAddress: 'yPDBTHAjPwJfZSSQYczccA78XRS2tZ5fZF',
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

    await expectDeterministicAppHash('5e4c517df4e8bdfb2ac742fe1a9d76a61cb231f86f311019bd87a26034951594');

    // New masternode identity should be created

    await expectMasternodeIdentity(
      newSmlFixture[0],
      transaction3,
      undefined,
      Address.fromString(newSmlFixture[0].payoutAddress),
    );

    // New operator should be created

    await expectOperatorIdentity(newSmlFixture[0]);

    // Masternode reward shares should be created

    const newMasternodeIdentifier = Identifier.from(
      Buffer.from(newSmlFixture[0].proRegTxHash, 'hex'),
    );

    const newOperatorIdentifier = createOperatorIdentifier(newSmlFixture[0]);

    const documentsResult = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', newMasternodeIdentifier],
          ['payToId', '==', newOperatorIdentifier],
        ],
      },
    );

    const documents = documentsResult.getValue();

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

    await expectDeterministicAppHash('6d73ee5a56bfdeddfb6479b41011d14e6454f265c5ac7c9a58e813b865d07714');

    // Masternode identity should stay

    await expectMasternodeIdentity(
      smlFixture[0],
      transaction1,
      Address.fromString(smlFixture[0].payoutAddress),
    );

    // Operator identity should stay

    await expectOperatorIdentity(smlFixture[0]);

    // Masternode reward shares should be removed

    const removedMasternodeIdentifier = Buffer.from(smlFixture[0].proRegTxHash, 'hex');

    const documentsResult = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', removedMasternodeIdentifier],
        ],
      },
    );

    const documents = documentsResult.getValue();

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

    await expectDeterministicAppHash('6d73ee5a56bfdeddfb6479b41011d14e6454f265c5ac7c9a58e813b865d07714');

    const invalidMasternodeIdentifier = Identifier.from(
      Buffer.from(invalidSmlEntry.proRegTxHash, 'hex'),
    );

    // Masternode reward shares should be removed

    const documentsResult = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', invalidMasternodeIdentifier],
        ],
      },
    );

    const documents = documentsResult.getValue();

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

    await expectDeterministicAppHash('62edcc5cf98ba354382b64995012f4ca391b5e77d519099aed91d0e40c7f660a');

    // Masternode identity should stay

    await expectMasternodeIdentity(
      smlFixture[0],
      transaction1,
      Address.fromString(smlFixture[0].payoutAddress),
    );

    // Previous operator identity should stay

    await expectOperatorIdentity(smlFixture[0]);

    // New operator identity should be created

    await expectOperatorIdentity(changedSmlEntry);

    // Only new masternode reward shares should exist

    const changedMasternodeIdentifier = Identifier.from(
      Buffer.from(changedSmlEntry.proRegTxHash, 'hex'),
    );

    const documentsResult = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', changedMasternodeIdentifier],
        ],
      },
    );

    const documents = documentsResult.getValue();

    expect(documents).to.have.lengthOf(1);

    const [document] = documents;

    const newOperatorIdentifier = createOperatorIdentifier(changedSmlEntry);

    expect(document.get('payToId')).to.deep.equal(newOperatorIdentifier);
  });

  it('should handle changed payout and operator payout addresses', async () => {
    // Sync initial list

    await synchronizeMasternodeIdentities(coreHeight);

    await expectDeterministicAppHash(firstSyncAppHash);

    // Mock SML

    const changedSmlEntry = smlFixture[0].copy();
    changedSmlEntry.payoutAddress = 'yMLrhooXyJtpV3R2ncsxvkrh6wRennNPoG';
    changedSmlEntry.operatorPayoutAddress = 'yT8DDY5NkX4ZtBkUVz7y1RgzbakCnMPogh';

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[1], changedSmlEntry] },
    );

    // Second call

    await synchronizeMasternodeIdentities(coreHeight + 1);

    await expectDeterministicAppHash('a750d62ba04e5a0771ca9d84524d8e57e835a3be4422b745d279031b6c4fc108');

    // Masternode identity should contain new public key

    await expectMasternodeIdentity(
      smlFixture[0],
      transaction1,
      Address.fromString(smlFixture[0].payoutAddress),
      Address.fromString(changedSmlEntry.payoutAddress),
    );

    // Previous operator identity should stay

    await expectOperatorIdentity(
      smlFixture[0],
      undefined,
      Address.fromString(changedSmlEntry.operatorPayoutAddress),
    );

    // New operator identity should be created

    await expectOperatorIdentity(
      changedSmlEntry,
      undefined,
      Address.fromString(changedSmlEntry.operatorPayoutAddress),
    );

    // Only new masternode reward shares should exist

    const changedMasternodeIdentifier = Identifier.from(
      Buffer.from(changedSmlEntry.proRegTxHash, 'hex'),
    );

    const documentsResult = await documentRepository.find(
      rewardsDataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', changedMasternodeIdentifier],
        ],
      },
    );

    const documents = documentsResult.getValue();

    expect(documents).to.have.lengthOf(1);

    const [document] = documents;

    const newOperatorIdentifier = createOperatorIdentifier(changedSmlEntry);

    expect(document.get('payToId')).to.deep.equal(newOperatorIdentifier);
  });
});
