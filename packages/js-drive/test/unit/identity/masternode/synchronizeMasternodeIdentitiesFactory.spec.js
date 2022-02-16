const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const { contractId } = require('@dashevo/masternode-reward-shares-contract/lib/systemIds');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const SimplifiedMNListEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListEntry');
const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const synchronizeMasternodeIdentitiesFactory = require('../../../../lib/identity/masternode/synchronizeMasternodeIdentitiesFactory');

describe('synchronizeMasternodeIdentitiesFactory', () => {
  let synchronizeMasternodeIdentities;
  let transactionalDppMock;
  let stateRepositoryMock;
  let simplifiedMasternodeListMock;
  let smlStoreMock;
  let coreHeight;
  let smlFixture;
  let masternodeRewardSharesContractId;
  let documentsFixture;
  let handleNewMasternodeMock;
  let handleUpdatedPubKeyOperatorMock;
  let splitDocumentsIntoChunksMock;
  let newSmlFixture;
  let stateTransitionFixture;
  let dataContractFixture;
  let dataContractRepositoryMock;
  let smlMaxListsLimit;
  let coreRpcClientMock;
  let rawDiff;

  beforeEach(function beforeEach() {
    smlMaxListsLimit = 16;
    stateTransitionFixture = getIdentityCreateTransitionFixture();
    documentsFixture = getDocumentsFixture();
    transactionalDppMock = createDPPMock(this.sinon);
    transactionalDppMock.document.createStateTransition.returns(stateTransitionFixture);

    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchDocuments.resolves([documentsFixture[0]]);

    simplifiedMasternodeListMock = {
      getStore: this.sinon.stub(),
    };

    masternodeRewardSharesContractId = Identifier.from(contractId);

    dataContractFixture = getDataContractFixture();

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

    newSmlFixture = new SimplifiedMNListEntry({
      proRegTxHash: '412aca2686194f0d01d64ec26ef502d8a9ba35f59de1a548a72b2bea60dcaf73',
      confirmedHash: '5f0d388b0c416ee7d328ca7b64f8c6d33d6312190935f406cb09bd60b8138551',
      service: '192.168.65.2:20201',
      pubKeyOperator: '0f648dab97de61672b880fb28b2aefdb3fb120c5a7157d9f9a80a8ded6d2cde031e318a8896fa624c55365fc7d7eea46',
      votingAddress: 'ybPTQPJsZpdrWeHLg4KPKNfH6fcvftt3sk',
      isValid: true,
    });

    smlStoreMock = {
      getSMLbyHeight: this.sinon.stub().returns({ mnList: smlFixture }),
      getCurrentSML: this.sinon.stub().returns({ mnList: smlFixture }),
    };

    simplifiedMasternodeListMock.getStore.returns(smlStoreMock);

    handleNewMasternodeMock = this.sinon.stub().returns({ create: [], delete: [] });
    handleUpdatedPubKeyOperatorMock = this.sinon.stub().returns({ create: [], delete: [] });

    splitDocumentsIntoChunksMock = this.sinon.stub().returns([{ create: [], delete: [] }]);

    dataContractRepositoryMock = {
      fetch: this.sinon.stub().resolves(dataContractFixture),
    };

    rawDiff = {
      baseBlockHash: '644bd9dcbc0537026af6d31181570f934d868f121c55513009bb36f509ec816e',
      blockHash: '23beac1b700c4a49855a9653e036219384ac2fab7eeba2ec45b3e2d0063d1285',
      cbTxMerkleTree: '03000000032f7f142e19bee0c595dac9f900695d1e428a4db70a805fda6c834cfec0de506a0d39baea39dbbaf9827a1f3b8f381a65ebcf4c2ef415025bc4d20afd372e680d12c226f084a6e28e421fbedff22b13aa1191d6a80744d104fa75ede12332467d0107',
      cbTx: '03000500010000000000000000000000000000000000000000000000000000000000000000ffffffff0502e9030101ffffffff01a2567a76070000001976a914f713c2fa5ef0e7c48f0d1b3ad2a79150037c72d788ac00000000460200e90300003fdbe53b9a4cd0b62284195cbd4f4c1655ebdd70e9117ed3c0e49c37bfce46060000000000000000000000000000000000000000000000000000000000000000',
      deletedMNs: [],
      mnList: [
        {
          proRegTxHash: 'e57402007ca10454d77437d9c1156b1c4ff8af86d699c08e9a31dbd1dfe3c991',
          confirmedHash: '0000000000000000000000000000000000000000000000000000000000000000',
          service: '127.0.0.1:20001',
          pubKeyOperator: '906d84cb88f532145d8838414f777b971c976ffcf8ccfc57413a13cf2f8a7750a92f9b997a5a741f1afa34d989f4312b',
          votingAddress: 'ydC3Qkhq6qc1qgHD8PVSHyAB6t3NYa7aw4',
          isValid: true,
        },
      ],
      deletedQuorums: [],
      newQuorums: [],
      merkleRootMNList: '0646cebf379ce4c0d37e11e970ddeb55164c4fbd5c198422b6d04c9a3be5db3f',
      merkleRootQuorums: '0000000000000000000000000000000000000000000000000000000000000000',
    };

    coreRpcClientMock = {
      protx: this.sinon.stub().resolves({
        result: rawDiff,
      }),
    };

    synchronizeMasternodeIdentities = synchronizeMasternodeIdentitiesFactory(
      transactionalDppMock,
      stateRepositoryMock,
      dataContractRepositoryMock,
      simplifiedMasternodeListMock,
      masternodeRewardSharesContractId,
      handleNewMasternodeMock,
      handleUpdatedPubKeyOperatorMock,
      splitDocumentsIntoChunksMock,
      smlMaxListsLimit,
      coreRpcClientMock,
    );

    coreHeight = 3;
  });

  it('should create identities for all masternodes on the first sync', async () => {
    await synchronizeMasternodeIdentities(coreHeight, false);

    expect(simplifiedMasternodeListMock.getStore).to.be.calledOnce();
    expect(smlStoreMock.getSMLbyHeight).to.be.calledOnceWithExactly(coreHeight);

    expect(handleNewMasternodeMock).to.be.calledTwice();
    expect(handleNewMasternodeMock.getCall(0)).to.be.calledWithExactly(
      smlFixture[0],
      dataContractFixture,
    );
    expect(handleNewMasternodeMock.getCall(1)).to.be.calledWithExactly(
      smlFixture[1],
      dataContractFixture,
    );

    expect(handleUpdatedPubKeyOperatorMock).to.be.not.called();

    expect(stateRepositoryMock.fetchDocuments).to.be.not.called();
    expect(transactionalDppMock.stateTransition.apply).to.be.not.called();
    expect(transactionalDppMock.document.createStateTransition).to.be.not.called();
    expect(splitDocumentsIntoChunksMock).to.be.not.called();

    expect(dataContractRepositoryMock.fetch).to.be.calledOnceWithExactly(
      masternodeRewardSharesContractId,
      true,
    );
    expect(coreRpcClientMock.protx).to.be.not.called();
  });

  it('should do nothing if nothing changed', async () => {
    await synchronizeMasternodeIdentities(coreHeight, false);

    await synchronizeMasternodeIdentities(coreHeight + 1, true);

    expect(handleNewMasternodeMock).to.be.calledTwice();
    expect(handleNewMasternodeMock.getCall(0)).to.be.calledWithExactly(
      smlFixture[0],
      dataContractFixture,
    );
    expect(handleNewMasternodeMock.getCall(1)).to.be.calledWithExactly(
      smlFixture[1],
      dataContractFixture,
    );

    expect(handleUpdatedPubKeyOperatorMock).to.be.not.called();

    expect(stateRepositoryMock.fetchDocuments).to.be.not.called();
    expect(splitDocumentsIntoChunksMock).to.be.not.called();
    expect(transactionalDppMock.document.createStateTransition).to.be.not.called();
    expect(transactionalDppMock.stateTransition.apply).to.be.not.called();
    expect(dataContractRepositoryMock.fetch).to.be.calledTwice();
    expect(coreRpcClientMock.protx).to.be.not.called();
  });

  it('should sync masternode identities if new masternode appeared', async () => {
    await synchronizeMasternodeIdentities(coreHeight, false);

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: smlFixture.concat(newSmlFixture) },
    );

    const newIdentities = { create: [documentsFixture[0]], delete: [] };

    handleNewMasternodeMock.returns(newIdentities);
    splitDocumentsIntoChunksMock.returns([newIdentities]);

    await synchronizeMasternodeIdentities(coreHeight + 1, true);

    expect(handleNewMasternodeMock).to.be.calledThrice();
    expect(handleNewMasternodeMock.getCall(0)).to.be.calledWithExactly(
      smlFixture[0],
      dataContractFixture,
    );
    expect(handleNewMasternodeMock.getCall(1)).to.be.calledWithExactly(
      smlFixture[1],
      dataContractFixture,
    );
    expect(handleNewMasternodeMock.getCall(2)).to.be.calledWithExactly(
      newSmlFixture,
      dataContractFixture,
    );

    expect(handleUpdatedPubKeyOperatorMock).to.be.not.called();
    expect(stateRepositoryMock.fetchDocuments).to.be.not.called();

    expect(splitDocumentsIntoChunksMock).to.be.calledWithExactly(newIdentities);
    expect(transactionalDppMock.document.createStateTransition)
      .to.be.calledWithExactly(newIdentities);
    expect(transactionalDppMock.stateTransition.apply).to.be.calledWithExactly(
      stateTransitionFixture,
    );
    expect(dataContractRepositoryMock.fetch).to.be.calledTwice();
    expect(coreRpcClientMock.protx).to.be.not.called();
  });

  it('should sync masternode identities if masternode disappeared', async () => {
    await synchronizeMasternodeIdentities(coreHeight, true);

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[0]] },
    );

    await synchronizeMasternodeIdentities(coreHeight + 1, true);

    expect(handleUpdatedPubKeyOperatorMock).to.be.not.called();

    expect(stateRepositoryMock.fetchDocuments).to.be.calledWithExactly(
      masternodeRewardSharesContractId,
      'masternodeRewardShares',
      {
        where: [
          ['$ownerId', '==', Identifier.from('XzhK3k3wuKfEsR6PBFPKf9BRpzLrXKcRHGHs5G6xgho')],
        ],
      },
    );
  });

  it('should sync masternode identities if masternode is not valid', async () => {
    await synchronizeMasternodeIdentities(coreHeight, true);

    const invalidSmlEntry = smlFixture[1];
    invalidSmlEntry.isValid = false;

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1, false).returns(
      { mnList: [smlFixture[0], invalidSmlEntry] },
    );

    await synchronizeMasternodeIdentities(coreHeight + 1);

    expect(handleUpdatedPubKeyOperatorMock).to.be.not.called();
    expect(stateRepositoryMock.fetchDocuments).to.be.calledWithExactly(
      masternodeRewardSharesContractId,
      'masternodeRewardShares',
      {
        where: [
          ['$ownerId', '==', Identifier.from('XzhK3k3wuKfEsR6PBFPKf9BRpzLrXKcRHGHs5G6xgho')],
        ],
      },
    );
  });

  it('should sync masternode identities if PubKeyOperator was changed', async () => {
    await synchronizeMasternodeIdentities(coreHeight, true);

    const changedSmlEntry = new SimplifiedMNListEntry(smlFixture[1]);
    changedSmlEntry.pubKeyOperator = newSmlFixture.pubKeyOperator;

    smlStoreMock.getSMLbyHeight.withArgs(coreHeight + 1).returns(
      { mnList: [smlFixture[0], changedSmlEntry] },
    );

    await synchronizeMasternodeIdentities(coreHeight + 1, false);

    expect(handleUpdatedPubKeyOperatorMock).to.be.calledOnceWithExactly(
      changedSmlEntry,
      smlFixture[1],
      dataContractFixture,
    );
    expect(stateRepositoryMock.fetchDocuments).to.be.not.called();
    expect(coreRpcClientMock.protx).to.be.not.called();
  });

  it('should fetch masternode list from core if the gap between coreHeight and lastSyncedCoreHeight > smlMaxListsLimit', async () => {
    await synchronizeMasternodeIdentities(coreHeight, true);

    await synchronizeMasternodeIdentities(coreHeight + smlMaxListsLimit + 1, true);

    expect(coreRpcClientMock.protx).to.be.calledOnceWithExactly('diff', 1, coreHeight);
  });
});
