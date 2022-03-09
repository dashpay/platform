const {
  asValue,
} = require('awilix');

const SimplifiedMNListEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListEntry');
const { hash } = require('@dashevo/dpp/lib/util/hash');
const { contractId } = require('@dashevo/masternode-reward-shares-contract/lib/systemIds');
const createTestDIContainer = require('../../../../lib/test/createTestDIContainer');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

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
  let identityRepository;
  let documentRepository;
  let dataContract;
  let dataContractRepository;

  beforeEach(async function beforeEach() {
    // rawDiff = {
    //   baseBlockHash: '644bd9dcbc0537026af6d31181570f934d868f121c55513009bb36f509ec816e',
    //   blockHash: '23beac1b700c4a49855a9653e036219384ac2fab7eeba2ec45b3e2d0063d1285',
    //   cbTxMerkleTree: '03000000032f7f142e19bee0c595dac9f900695d1e428a4db70a805fda6c834cfec0de506a0d39baea39dbbaf9827a1f3b8f381a65ebcf4c2ef415025bc4d20afd372e680d12c226f084a6e28e421fbedff22b13aa1191d6a80744d104fa75ede12332467d0107',
    //   cbTx: '03000500010000000000000000000000000000000000000000000000000000000000000000ffffffff0502e9030101ffffffff01a2567a76070000001976a914f713c2fa5ef0e7c48f0d1b3ad2a79150037c72d788ac00000000460200e90300003fdbe53b9a4cd0b62284195cbd4f4c1655ebdd70e9117ed3c0e49c37bfce46060000000000000000000000000000000000000000000000000000000000000000',
    //   deletedMNs: [],
    //   mnList: [
    //     {
    //       proRegTxHash: 'e57402007ca10454d77437d9c1156b1c4ff8af86d699c08e9a31dbd1dfe3c991',
    //       confirmedHash: '0000000000000000000000000000000000000000000000000000000000000000',
    //       service: '127.0.0.1:20001',
    //       pubKeyOperator: '906d84cb88f532145d8838414f777b971c976ffcf8ccfc57413a13cf2f8a7750a92f9b997a5a741f1afa34d989f4312b',
    //       votingAddress: 'ydC3Qkhq6qc1qgHD8PVSHyAB6t3NYa7aw4',
    //       isValid: true,
    //     },
    //   ],
    //   deletedQuorums: [],
    //   newQuorums: [],
    //   merkleRootMNList: '0646cebf379ce4c0d37e11e970ddeb55164c4fbd5c198422b6d04c9a3be5db3f',
    //   merkleRootQuorums: '0000000000000000000000000000000000000000000000000000000000000000',
    // };

    coreHeight = 3;

    container = await createTestDIContainer();

    // Mock Core

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
    identityRepository = container.resolve('identityRepository');
    documentRepository = container.resolve('documentRepository');
    dataContractRepository = container.resolve('dataContractRepository');

    dataContract = await dataContractRepository.fetch(Identifier.from(contractId));

    await registerSystemDataContract(
      masternodeRewardSharesOwnerId,
      masternodeRewardSharesContractId,
      masternodeRewardSharesOwnerPublicKey,
      masternodeRewardSharesDocuments,
    );

    synchronizeMasternodeIdentities = container.resolve('synchronizeMasternodeIdentities');
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should create identities for all masternodes on the first sync', async () => {
    await synchronizeMasternodeIdentities(coreHeight);

    const firstIdentifier = hash(
      Buffer.from(smlFixture[0].proRegTxHash, 'hex'),
    );
    const firstIdentity = await identityRepository.fetch(Identifier.from(firstIdentifier));

    expect(firstIdentity).to.exist();

    const secondIdentifier = hash(
      Buffer.from(smlFixture[1].proRegTxHash, 'hex'),
    );

    const secondIdentity = await identityRepository.fetch(Identifier.from(secondIdentifier));

    expect(secondIdentity).to.exist();

    const firstOperatorPubKey = Buffer.from(smlFixture[0].pubKeyOperator, 'hex');

    const firstOperatorIdentityId = hash(
      Buffer.concat([
        Buffer.from(smlFixture[0].proRegTxHash, 'hex'),
        firstOperatorPubKey,
      ]),
    );

    let documents = await documentRepository.find(
      dataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', firstIdentifier],
          ['payToId', '==', firstOperatorIdentityId],
        ],
      },
    );

    expect(documents).to.have.lengthOf(1);

    expect(documents[0].getOwnerId()).to.deep.equal(firstIdentifier);
    expect(documents[0].getData().percentage).to.equal(100);
    expect(documents[0].getData().payToId).to.deep.equal(firstOperatorIdentityId);

    const secondOperatorPubKey = Buffer.from(smlFixture[1].pubKeyOperator, 'hex');

    const secondOperatorIdentityId = hash(
      Buffer.concat([
        Buffer.from(smlFixture[1].proRegTxHash, 'hex'),
        secondOperatorPubKey,
      ]),
    );

    documents = await documentRepository.find(
      dataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', secondIdentifier],
          ['payToId', '==', secondOperatorIdentityId],
        ],
      },
    );

    expect(documents).to.have.lengthOf(0);
  });

  it.skip('should sync identities if the gap between coreHeight and lastSyncedCoreHeight > smlMaxListsLimit', async () => {
    //coreRpcClient

    coreRpcClientMock = {
      protx: this.sinon.stub().resolves({
        result: rawDiff,
      }),
    };

    await synchronizeMasternodeIdentities(coreHeight);

    await synchronizeMasternodeIdentities(coreHeight + smlMaxListsLimit + 1);

  });

  it('should create masternode identities if new masternode appeared', async function it() {
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

    // Mock Core

    const transaction3 = {
      extraPayload: {
        operatorReward: 200,
        keyIDOwner: Buffer.alloc(20).fill('c').toString('hex'),
      },
    };

    fetchTransactionMock.withArgs('3b73b21f45b216dce2b4ffb4a85e1471d57aed6bf8e34d961a48296fe9b7f53b').resolves(transaction3);

    // Second call

    await synchronizeMasternodeIdentities(coreHeight + 1);

    const newIdentifier = hash(
      Buffer.from(newSmlFixture.proRegTxHash, 'hex'),
    );
    const newIdentity = await transactionalStateRepository.fetchIdentity(newIdentifier);

    expect(newIdentity).to.exist();
  });

  it('should remove reward shares if masternode disappeared', async () => {
    // Sync initial list

    await synchronizeMasternodeIdentities(coreHeight);

    // Mock SML

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[0]] },
    );

    // delete smlFixture[1];

    // Second call

    await synchronizeMasternodeIdentities(coreHeight + 1);
    const removedIdentifier = hash(
      Buffer.from(smlFixture[0].proRegTxHash, 'hex'),
    );
    const removedIdentity = await transactionalStateRepository.fetchIdentity(removedIdentifier);

    expect(removedIdentity).to.be.null();

    const documents = await transactionalStateRepository.fetchDocuments(
      contractId,
      'rewardShare',
      { where: [['$id', '==', rewardShare.getId()]] },
    );

  });

  it('should remove reward shares if masternode is not valid', async () => {
    // Sync initial list

    await synchronizeMasternodeIdentities(coreHeight);

    // Mock SML

    const invalidSmlEntry = smlFixture[1];
    invalidSmlEntry.isValid = false;

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1, false).returns(
      { mnList: [smlFixture[0], invalidSmlEntry] },
    );

    // Second call

    await synchronizeMasternodeIdentities(coreHeight + 1);

    const removedIdentifier = hash(
      Buffer.from(smlFixture[0].proRegTxHash, 'hex'),
    );
    const removedIdentity = await transactionalStateRepository.fetchIdentity(removedIdentifier);

    expect(removedIdentity).to.be.null();
  });

  it('should update create operator identity and reward shares if PubKeyOperator was changed', async () => {
    // Initial sync

    await synchronizeMasternodeIdentities(coreHeight, true);

    const changedSmlEntry = new SimplifiedMNListEntry(smlFixture[1]);
    changedSmlEntry.pubKeyOperator = newSmlFixture.pubKeyOperator;

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[0], changedSmlEntry] },
    );

    smlFixture[1].pubKeyOperator = 'cca9789fab00deae1464ed80bda281fc833f85959b04201645e5fc25635e3e7ecda30d13d328b721af0809fca3bf3bcc'

    await synchronizeMasternodeIdentities(coreHeight + 1, false);

    const removedIdentifier = hash(
      Buffer.from(smlFixture[0].proRegTxHash, 'hex'),
    );
    const removedIdentity = await transactionalStateRepository.fetchIdentity(removedIdentifier);

    expect(removedIdentity).to.be.null();
  });
});
