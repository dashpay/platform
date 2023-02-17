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
const BlockInfo = require('../../../../lib/blockExecution/BlockInfo');
const createVotingIdentifier = require('../../../../lib/identity/masternode/createVotingIdentifier');
const getSystemIdentityPublicKeysFixture = require('../../../../lib/test/fixtures/getSystemIdentityPublicKeysFixture');

/**
 * @param {IdentityStoreRepository} identityRepository
 * @param {IdentityPublicKeyStoreRepository} identityPublicKeyRepository
 * @param {getWithdrawPubKeyTypeFromPayoutScript} getWithdrawPubKeyTypeFromPayoutScript
 * @param {getPublicKeyFromPayoutScript} getPublicKeyFromPayoutScript
 * @returns {expectOperatorIdentity}
 */
function expectOperatorIdentityFactory(
  identityRepository,
  identityPublicKeyRepository,
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

    const operatorIdentityResult = await identityRepository.fetch(
      operatorIdentifier,
      { useTransaction: true },
    );

    const operatorIdentity = operatorIdentityResult.getValue();

    expect(operatorIdentity).to.exist();

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

    const firstOperatorIdentityByPublicKeyHashResult = await identityRepository
      .fetchByPublicKeyHash(firstOperatorMasternodePublicKey.hash(), { useTransaction: true });

    const firstOperatorIdentityByPublicKeyHash = firstOperatorIdentityByPublicKeyHashResult
      .getValue();

    expect(firstOperatorIdentityByPublicKeyHash).to.be.not.null();
    expect(firstOperatorIdentityByPublicKeyHash.getId())
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

      const masternodeIdentityByPayoutPublicKeyHashResult = await identityPublicKeyRepository
        .fetch(payoutPublicKey.hash(), { useTransaction: true });

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

      const masternodeIdentityByPayoutPublicKeyHashResult = await identityRepository
        .fetchByPublicKeyHash(payoutPublicKey.hash(), { useTransaction: true });

      const masternodeIdentityByPayoutPublicKeyHash = masternodeIdentityByPayoutPublicKeyHashResult
        .getValue();

      expect(masternodeIdentityByPayoutPublicKeyHash).to.be.not.null();
      expect(masternodeIdentityByPayoutPublicKeyHash.getId())
        .to.deep.equal(operatorIdentifier);
    }
  }

  return expectOperatorIdentity;
}

/**
 * @param {IdentityStoreRepository} identityRepository
 * @returns {expectVotingIdentity}
 */
function expectVotingIdentityFactory(
  identityRepository,
) {
  /**
   * @typedef {expectVotingIdentity}
   * @param {SimplifiedMNListEntry} smlEntry
   * @param {Buffer} proRegTx
   * @returns {Promise<void>}
   */
  async function expectVotingIdentity(
    smlEntry,
    proRegTx,
  ) {
    // Validate voting identity

    const votingIdentifier = createVotingIdentifier(smlEntry);

    const votingIdentityResult = await identityRepository.fetch(votingIdentifier, {
      useTransaction: true,
    });

    const votingIdentity = votingIdentityResult.getValue();

    expect(votingIdentity)
      .to
      .exist();

    // Validate voting public keys

    expect(votingIdentity.getPublicKeys())
      .to
      .have
      .lengthOf(1);

    const masternodePublicKey = votingIdentity.getPublicKeyById(0);
    expect(masternodePublicKey.getType()).to.equal(IdentityPublicKey.TYPES.ECDSA_HASH160);
    expect(masternodePublicKey.getData()).to.deep.equal(
      Buffer.from(proRegTx.extraPayload.keyIDVoting, 'hex').reverse(),
    );

    const masternodeIdentityByPublicKeyHashResult = await identityRepository
      .fetchByPublicKeyHash(masternodePublicKey.hash(), {
        useTransaction: true,
      });

    const masternodeIdentityByPublicKeyHash = masternodeIdentityByPublicKeyHashResult.getValue();

    expect(masternodeIdentityByPublicKeyHash).to.be.not.null();
    expect(masternodeIdentityByPublicKeyHash.getId())
      .to.deep.equal(votingIdentifier);
  }

  return expectVotingIdentity;
}

/**
 * @param {IdentityStoreRepository} identityRepository
 * @param {IdentityPublicKeyStoreRepository} identityPublicKeyRepository
 * @param {getWithdrawPubKeyTypeFromPayoutScript} getWithdrawPubKeyTypeFromPayoutScript
 * @param {getPublicKeyFromPayoutScript} getPublicKeyFromPayoutScript
 * @returns {expectMasternodeIdentity}
 */
function expectMasternodeIdentityFactory(
  identityRepository,
  identityPublicKeyRepository,
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

    const masternodeIdentityResult = await identityRepository.fetch(
      masternodeIdentifier,
      { useTransaction: true },
    );

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

    const masternodeIdentityByPublicKeyHashResult = await identityRepository
      .fetchManyByPublicKeyHashes([masternodePublicKey.hash()], { useTransaction: true });

    const masternodeIdentityByPublicKeyHash = masternodeIdentityByPublicKeyHashResult.getValue();

    expect(masternodeIdentityByPublicKeyHash).to.have.lengthOf(1);
    expect(masternodeIdentityByPublicKeyHash[0].getId())
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

      const masternodeIdentityByPayoutPublicKeyHashResult = await identityRepository
        .fetchByPublicKeyHash(payoutPublicKey.hash(), { useTransaction: true });

      const masternodeIdentityByPayoutPublicKeyHash = masternodeIdentityByPayoutPublicKeyHashResult
        .getValue();

      expect(masternodeIdentityByPayoutPublicKeyHash).to.not.be.null();
      expect(masternodeIdentityByPayoutPublicKeyHash.getId())
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

      const masternodeIdentityByPayoutPublicKeyHashResult = await identityRepository
        .fetchByPublicKeyHash(payoutPublicKey.hash(), { useTransaction: true });

      const masternodeIdentityByPayoutPublicKeyHash = masternodeIdentityByPayoutPublicKeyHashResult
        .getValue();

      expect(masternodeIdentityByPayoutPublicKeyHash).to.not.be.null();
      expect(masternodeIdentityByPayoutPublicKeyHash.getId())
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

describe('synchronizeMasternodeIdentitiesFactory', function main() {
  this.timeout(10000);
  let container;
  let coreHeight;
  let fetchSimplifiedMNListMock;
  let fetchedSimplifiedMNList;
  let fetchTransactionMock;
  let smlStoreMock;
  let smlFixture;
  let newSmlFixture;
  let transaction1;
  let transaction2;
  let transaction3;
  let synchronizeMasternodeIdentities;
  let rewardsDataContract;
  let identityRepository;
  let documentRepository;
  let identityPublicKeyRepository;
  let expectOperatorIdentity;
  let expectVotingIdentity;
  let expectMasternodeIdentity;
  let expectDeterministicAppHash;
  let firstSyncAppHash;
  let blockInfo;

  beforeEach(async function beforeEach() {
    coreHeight = 3;
    firstSyncAppHash = '3faf41663866d5a66eb73c504861136233e2b10ae60c459f39573726786b2bd9';
    blockInfo = new BlockInfo(10, 0, 1668702100799);

    container = await createTestDIContainer();

    // Mock Core RPC

    fetchedSimplifiedMNList = {
      mnList: [],
    };

    fetchSimplifiedMNListMock = this.sinon.stub().resolves(fetchedSimplifiedMNList);

    container.register('fetchSimplifiedMNList', asValue(fetchSimplifiedMNListMock));

    // Mock SML

    smlFixture = [
      new SimplifiedMNListEntry({
        proRegTxHash: 'a2c9b34ef525271d84f70a0d4d2c107e8a2f81cd4d8256dc7b3911ed253d5611',
        confirmedHash: '29ff8afb463604ba7d984b483e92dfefa4e80e12de3acae6d75f9b910df9eab6',
        service: '192.168.65.2:20201',
        pubKeyOperator: 'a5ad6d8cad7b233210b718a5fc9ec3cea18aeebe38b2e3122deb581e430aa28875fe7336c283871db42808f8d4107745',
        votingAddress: 'yRXtaRmQ7LCmT5XcgzQdLwPEf31dycBaeY',
        isValid: true,
        payoutAddress: 'yR843jN58m5dubmQjfUmKDDJMJzNatFV9M',
        payoutOperatorAddress: 'yNjsnYM16J5NZPA2P8BKJG3MKfUD7XHAFE',
        nType: 0,
      }),
      new SimplifiedMNListEntry({
        proRegTxHash: 'f5ec54aed788c434da2fc535ea6b125ec6fc54e58bc0a00a005d1a8d5e477a90',
        confirmedHash: '53125505b0e9d11b371cf3e12c92d164296dfa215fde6201d28ea44bed992187',
        service: '192.168.65.2:20101',
        pubKeyOperator: '951a3208ba531ea75aedd2dc0a9efc75f2c4d9492f1ee0a989b593bcd9722b1a101774d80a426552a9f91d24eb55af6e',
        votingAddress: 'yYH1rgZsgvkmT8bSSSw1cKCjyVPnFpTBCw',
        isValid: true,
        payoutAddress: 'ycL7L4mhYoaZdm9TH85svvpfeKtdfo249u',
        nType: 0,
      }),
    ];

    newSmlFixture = [
      new SimplifiedMNListEntry({
        proRegTxHash: '1c81a5faa2c0e0d96eb59c58a10fcbc87f431bb6cd880d960b43b269e682d2d2',
        confirmedHash: '03cc2acc135ab51304d3cff42215c7a8041902fa3f19451d5562a03b38143e8f',
        service: '192.168.65.2:20001',
        pubKeyOperator: '96f83eedc8a7b87663e591987f051ce341a6fb88989322c64bbbf56d205e4e77d2cb7d839d8b4106a8a1f5d5cf7cfa57',
        votingAddress: 'ybJfuKs59MJWkPEnS8qNmtvdisHrCy7Njn',
        isValid: true,
        payoutAddress: '7UkJidhNjEPJCQnCTXeaJKbJmL4JuyV66w',
        payoutOperatorAddress: 'yPDBTHAjPwJfZSSQYczccA78XRS2tZ5fZF',
        nType: 0,
      }),
    ];

    smlStoreMock = {
      getSMLbyHeight: this.sinon.stub().returns({ mnList: smlFixture }),
    };

    const simplifiedMasternodeListMock = {
      getStore: this.sinon.stub().returns(smlStoreMock),
    };

    container.register('simplifiedMasternodeList', asValue(simplifiedMasternodeListMock));

    // Mock fetchTransaction

    fetchTransactionMock = this.sinon.stub();

    transaction1 = {
      extraPayload: {
        operatorReward: 100,
        keyIDOwner: Buffer.alloc(20).fill('a').toString('hex'),
        keyIDVoting: Buffer.alloc(20).fill('b').toString('hex'),
      },
    };

    transaction2 = {
      extraPayload: {
        operatorReward: 0,
        keyIDOwner: Buffer.alloc(20).fill('c').toString('hex'),
        keyIDVoting: Buffer.alloc(20).fill('d').toString('hex'),
      },
    };

    transaction3 = {
      extraPayload: {
        operatorReward: 200,
        keyIDOwner: Buffer.alloc(20).fill('e').toString('hex'),
        keyIDVoting: Buffer.alloc(20).fill('f').toString('hex'),
      },
    };

    fetchTransactionMock.withArgs('a2c9b34ef525271d84f70a0d4d2c107e8a2f81cd4d8256dc7b3911ed253d5611').resolves(transaction1);
    fetchTransactionMock.withArgs('f5ec54aed788c434da2fc535ea6b125ec6fc54e58bc0a00a005d1a8d5e477a90').resolves(transaction2);
    fetchTransactionMock.withArgs('1c81a5faa2c0e0d96eb59c58a10fcbc87f431bb6cd880d960b43b269e682d2d2').resolves(transaction3);

    container.register('fetchTransaction', asValue(fetchTransactionMock));

    const groveDBStore = container.resolve('groveDBStore');
    await groveDBStore.startTransaction();

    /**
     * @type {Drive}
     */
    const rsDrive = container.resolve('rsDrive');
    await rsDrive.getAbci().initChain({
      genesisTimeMs: 0,
      systemIdentityPublicKeys: getSystemIdentityPublicKeysFixture(),
    }, true);

    const masternodeRewardSharesContractId = container.resolve('masternodeRewardSharesContractId');

    [rewardsDataContract] = await rsDrive.fetchContract(masternodeRewardSharesContractId, 0, true);

    /**
     * @type {synchronizeMasternodeIdentities}
     */
    synchronizeMasternodeIdentities = container.resolve('synchronizeMasternodeIdentities');

    identityRepository = container.resolve('identityRepository');
    documentRepository = container.resolve('documentRepository');
    identityPublicKeyRepository = container.resolve('identityPublicKeyRepository');
    const getWithdrawPubKeyTypeFromPayoutScript = container.resolve('getWithdrawPubKeyTypeFromPayoutScript');
    const getPublicKeyFromPayoutScript = container.resolve('getPublicKeyFromPayoutScript');

    expectOperatorIdentity = expectOperatorIdentityFactory(
      identityRepository,
      identityPublicKeyRepository,
      getWithdrawPubKeyTypeFromPayoutScript,
      getPublicKeyFromPayoutScript,
    );

    expectVotingIdentity = expectVotingIdentityFactory(
      identityRepository,
    );

    expectMasternodeIdentity = expectMasternodeIdentityFactory(
      identityRepository,
      identityPublicKeyRepository,
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
    const result = await synchronizeMasternodeIdentities(coreHeight, blockInfo);

    expect(result.fromHeight).to.be.equal(0);
    expect(result.toHeight).to.be.equal(3);
    expect(result.createdEntities).to.have.lengthOf(6);
    expect(result.updatedEntities).to.have.lengthOf(0);
    expect(result.removedEntities).to.have.lengthOf(0);

    await expectDeterministicAppHash(firstSyncAppHash);

    /**
     * Validate first masternode
     */

    // Masternode identity should be created

    await expectMasternodeIdentity(
      smlFixture[0],
      transaction1,
      undefined,
      Address.fromString(smlFixture[0].payoutAddress),
    );

    // voting identity should be created
    await expectVotingIdentity(
      smlFixture[0],
      transaction1,
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
        useTransaction: true,
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

    // Voting identity should be created
    await expectVotingIdentity(
      smlFixture[1],
      transaction2,
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

    const secondOperatorIdentityResult = await identityRepository.fetch(
      secondOperatorIdentifier,
      { useTransaction: true },
    );

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
        useTransaction: true,
      },
    );

    documents = documentsResult.getValue();

    expect(documents).to.have.lengthOf(0);
  });

  it('should sync identities if the gap between coreHeight and lastSyncedCoreHeight > smlMaxListsLimit', async () => {
    // Sync initial list

    await synchronizeMasternodeIdentities(coreHeight, blockInfo);

    await expectDeterministicAppHash(firstSyncAppHash);

    const nextCoreHeight = coreHeight + 42;

    // Mock SML

    smlStoreMock.getSMLbyHeight.withArgs(nextCoreHeight).returns({
      mnList: smlFixture.concat(newSmlFixture),
    });

    fetchedSimplifiedMNList.mnList = smlFixture;

    // Second call

    const result = await synchronizeMasternodeIdentities(nextCoreHeight, blockInfo);

    expect(result.fromHeight).to.be.equal(3);
    expect(result.toHeight).to.be.equal(45);
    expect(result.createdEntities).to.have.lengthOf(4);
    expect(result.updatedEntities).to.have.lengthOf(0);
    expect(result.removedEntities).to.have.lengthOf(0);

    // Nothing happened

    await expectDeterministicAppHash('1bc066ff1f1ab91cac098ebdfb396f435336e6e3337dabf13c7bc3757a591507');

    // Core RPC should be called

    expect(fetchSimplifiedMNListMock).to.have.been.calledOnceWithExactly(1, coreHeight);
  });

  it('should create masternode identities if new masternode appeared', async () => {
    // Sync initial list

    const result = await synchronizeMasternodeIdentities(coreHeight, blockInfo);

    expect(result.fromHeight).to.be.equal(0);
    expect(result.toHeight).to.be.equal(coreHeight);
    expect(result.createdEntities).to.have.lengthOf(6);
    expect(result.updatedEntities).to.have.lengthOf(0);
    expect(result.removedEntities).to.have.lengthOf(0);

    await expectDeterministicAppHash(firstSyncAppHash);

    // Mock SML

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: smlFixture.concat(newSmlFixture) },
    );

    // Second call

    const result2 = await synchronizeMasternodeIdentities(coreHeight + 1, blockInfo);

    expect(result2.fromHeight).to.be.equal(3);
    expect(result2.toHeight).to.be.equal(4);
    expect(result2.createdEntities).to.have.lengthOf(4);
    expect(result2.updatedEntities).to.have.lengthOf(0);
    expect(result2.removedEntities).to.have.lengthOf(0);

    await expectDeterministicAppHash('ca0cee6a797d8707cfd48277b2421573f883f38b6c89b3debd47fe0b0cb195eb');

    // New masternode identity should be created

    await expectMasternodeIdentity(
      newSmlFixture[0],
      transaction3,
      undefined,
      Address.fromString(newSmlFixture[0].payoutAddress),
    );

    // New voting identity should be created

    await expectVotingIdentity(
      newSmlFixture[0],
      transaction3,
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
        useTransaction: true,
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

    await synchronizeMasternodeIdentities(coreHeight, blockInfo);

    await expectDeterministicAppHash(firstSyncAppHash);

    // Mock SML

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[1]] },
    );

    // Second call

    const result = await synchronizeMasternodeIdentities(coreHeight + 1, blockInfo);

    expect(result.fromHeight).to.be.equal(3);
    expect(result.toHeight).to.be.equal(4);
    expect(result.createdEntities).to.have.lengthOf(0);
    expect(result.updatedEntities).to.have.lengthOf(0);
    expect(result.removedEntities).to.have.lengthOf(1);

    await expectDeterministicAppHash('f9b6d7f3358b6e2ab4d6bcddf41592ad93cc07f0d3a7d6664f383e261e84f5e7');

    // Masternode identity should stay

    await expectMasternodeIdentity(
      smlFixture[0],
      transaction1,
      undefined,
      Address.fromString(smlFixture[0].payoutAddress),
    );

    // Voting identity should stay

    await expectVotingIdentity(
      smlFixture[0],
      transaction1,
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
        useTransaction: true,
      },
    );

    const documents = documentsResult.getValue();

    expect(documents).to.have.lengthOf(0);
  });

  it('should remove reward shares if masternode is not valid', async () => {
    // Sync initial list

    await synchronizeMasternodeIdentities(coreHeight, blockInfo);

    await expectDeterministicAppHash(firstSyncAppHash);

    // Mock SML

    const invalidSmlEntry = smlFixture[0].copy();
    invalidSmlEntry.isValid = false;

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[1], invalidSmlEntry] },
    );

    // Second call

    const result = await synchronizeMasternodeIdentities(coreHeight + 1, blockInfo);

    expect(result.fromHeight).to.be.equal(3);
    expect(result.toHeight).to.be.equal(4);
    expect(result.createdEntities).to.have.lengthOf(0);
    expect(result.updatedEntities).to.have.lengthOf(0);
    expect(result.removedEntities).to.have.lengthOf(1);

    await expectDeterministicAppHash('f9b6d7f3358b6e2ab4d6bcddf41592ad93cc07f0d3a7d6664f383e261e84f5e7');

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
        useTransaction: true,
      },
    );

    const documents = documentsResult.getValue();

    expect(documents).to.have.lengthOf(0);
  });

  it('should create operator identity and reward shares if PubKeyOperator was changed', async () => {
    // Initial sync

    await synchronizeMasternodeIdentities(coreHeight, blockInfo);

    await expectDeterministicAppHash(firstSyncAppHash);

    // Mock SML

    const changedSmlEntry = smlFixture[0].copy();
    changedSmlEntry.pubKeyOperator = '96f83eedc8a7b87663e591987f051ce341a6fb88989322c64bbbf56d205e4e77d2cb7d839d8b4106a8a1f5d5cf7cfa57';

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[1], changedSmlEntry] },
    );

    // Second call

    const result = await synchronizeMasternodeIdentities(coreHeight + 1, blockInfo);

    expect(result.fromHeight).to.be.equal(3);
    expect(result.toHeight).to.be.equal(4);
    expect(result.createdEntities).to.have.lengthOf(2);
    expect(result.updatedEntities).to.have.lengthOf(1);
    expect(result.removedEntities).to.have.lengthOf(1);

    await expectDeterministicAppHash('a582e896e609e333b2e0b208040377330d8114031429c09c5ae7fc21b21573b9');

    // Masternode identity should stay

    await expectMasternodeIdentity(
      smlFixture[0],
      transaction1,
      undefined,
      Address.fromString(smlFixture[0].payoutAddress),
    );

    // Previous voting identity should stay

    await expectVotingIdentity(
      smlFixture[0],
      transaction1,
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
        useTransaction: true,
      },
    );

    const documents = documentsResult.getValue();

    expect(documents).to.have.lengthOf(1);

    const [document] = documents;

    const newOperatorIdentifier = createOperatorIdentifier(changedSmlEntry);

    expect(document.get('payToId')).to.deep.equal(newOperatorIdentifier);
  });

  it('should handle changed payout, voting and operator payout addresses', async () => {
    // Sync initial list

    await synchronizeMasternodeIdentities(coreHeight, blockInfo);

    await expectDeterministicAppHash(firstSyncAppHash);

    // Mock SML

    const changedSmlEntry = smlFixture[0].copy();
    changedSmlEntry.payoutAddress = 'yMLrhooXyJtpV3R2ncsxvkrh6wRennNPoG';
    changedSmlEntry.operatorPayoutAddress = 'yT8DDY5NkX4ZtBkUVz7y1RgzbakCnMPogh';

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[1], changedSmlEntry] },
    );

    // Second call

    await synchronizeMasternodeIdentities(coreHeight + 1, blockInfo);

    await expectDeterministicAppHash('1f6ff4c65a9eee17877dfdc20c9569c47d4b0d901f2a40ea933fcc6a852055b8');

    // Masternode identity should contain new public key

    await expectMasternodeIdentity(
      smlFixture[0],
      transaction1,
      Address.fromString(smlFixture[0].payoutAddress),
      Address.fromString(changedSmlEntry.payoutAddress),
    );

    // Previous voting identity should stay

    await expectVotingIdentity(
      smlFixture[0],
      transaction1,
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

    // new voting Identity should exist
    await expectVotingIdentity(
      changedSmlEntry,
      transaction1,
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
        useTransaction: true,
      },
    );

    const documents = documentsResult.getValue();

    expect(documents).to.have.lengthOf(1);

    const [document] = documents;

    const newOperatorIdentifier = createOperatorIdentifier(changedSmlEntry);

    expect(document.get('payToId')).to.deep.equal(newOperatorIdentifier);
  });

  it('should not create voting Identity if owner and voting keys are the same', async () => {
    transaction1 = {
      extraPayload: {
        operatorReward: 100,
        keyIDOwner: Buffer.alloc(20).fill('a').toString('hex'),
        keyIDVoting: Buffer.alloc(20).fill('a').toString('hex'),
      },
    };

    fetchTransactionMock.withArgs('a2c9b34ef525271d84f70a0d4d2c107e8a2f81cd4d8256dc7b3911ed253d5611').resolves(transaction1);

    // Initial sync

    await synchronizeMasternodeIdentities(coreHeight, blockInfo);
    await expectDeterministicAppHash('a57d1dfac825f23f518f5d48d41f9e786ac7018da4993cdf5255c27846bfac55');
    const votingIdentifier = createVotingIdentifier(smlFixture[0]);

    const votingIdentityResult = await identityRepository.fetch(
      votingIdentifier,
      { useTransaction: true },
    );

    expect(votingIdentityResult.isNull()).to.be.true();
  });
});
