const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const { contractId } = require('@dashevo/masternode-reward-shares-contract/lib/systemIds');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const SimplifiedMNListEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListEntry');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const handleUpdatedPubKeyOperatorFactory = require('../../../../lib/identity/masternode/handleUpdatedPubKeyOperatorFactory');

describe('handleUpdatedPubKeyOperatorFactory', () => {
  let handleUpdatedPubKeyOperator;
  let transactionalDppMock;
  let stateRepositoryMock;
  let createMasternodeIdentityMock;
  let masternodeRewardSharesContractId;
  let documentsFixture;
  let rawTransactionFixture;
  let masternodeEntry;
  let previousMasternodeEntry;
  let identityFixture;
  let dataContractFixture;

  beforeEach(function beforeEach() {
    dataContractFixture = getDataContractFixture();
    identityFixture = getIdentityFixture();

    rawTransactionFixture = '03000100018dee5838d18e38e62436fec18f4df9159e72af98b2bdff967ba9594962167d66000000006b483045022100e240a14a286fc575d5b7fd2359ebd94b44ddb6abe6e328a5a9055d73b30608a90220542071ba0ec1476bcf3e97091f5e3d73c424e2d11ac7c9471a43115112ebb4ec012103644d63815114a4ba0f2add003278ee6a8e13ce0283ab5bc61594cc4a75930475feffffff01a1949800000000001976a9143359e023b6d6f161bb2727f1ae9053a5b1eaedd988ac00000000fd120101000000000086842e2d813096e76cd8193bbdc90b7e829fa4266f5370aa246f9cfe9299d9f10000000000000000000000000000ffffc0a841024e85d095b3e04f3004bbb4a8567c765ed169afbc36418e4c8c144bd6c62640fe3ae295973d512f83f7f541525a5da3c91e77ec02ff4dcd214e7431b7d2cc28e420ebfeb612eed095b3e04f3004bbb4a8567c765ed169afbc364100001976a9143359e023b6d6f161bb2727f1ae9053a5b1eaedd988ac546183d65046070847ac8ce38e871d7135068423a23f6f79e8b2a10b15998add4120d5a944b6bb0b885389a6e9fbc1d430021cb7a9489b07b2538b8cca5f017ac9535a1cc4e9e575b7560b41ab7d9e01e835c7acf71c647fb1ac947f6bac5fbc8554';

    documentsFixture = getDocumentsFixture();

    transactionalDppMock = createDPPMock(this.sinon);
    transactionalDppMock.document.create.returns(documentsFixture[0]);
    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchTransaction.resolves({
      data: rawTransactionFixture,
      height: 626,
    });
    stateRepositoryMock.fetchIdentity.resolves(
      identityFixture,
    );
    stateRepositoryMock.fetchDocuments.resolves([documentsFixture[1]]);

    createMasternodeIdentityMock = this.sinon.stub();
    masternodeRewardSharesContractId = Identifier.from(contractId);

    handleUpdatedPubKeyOperator = handleUpdatedPubKeyOperatorFactory(
      transactionalDppMock,
      stateRepositoryMock,
      createMasternodeIdentityMock,
      masternodeRewardSharesContractId,
    );

    masternodeEntry = new SimplifiedMNListEntry({
      proRegTxHash: '954112bb018895896cfa3c3d00761a045fc16b22f2170c1fbb029a2936c68f16',
      confirmedHash: '1de71625dbc973e2377ebd7da4fe6f8a8eb8af8c5a99373e36151a4fbe9947cc',
      service: '192.168.65.2:20101',
      pubKeyOperator: '8e4c8c144bd6c62640fe3ae295973d512f83f7f541525a5da3c91e77ec02ff4dcd214e7431b7d2cc28e420ebfeb612ee',
      votingAddress: 'yfLLjdEynGQBdoPcCDUNAxu6pksYGzXKA4',
      isValid: true,
    });

    previousMasternodeEntry = new SimplifiedMNListEntry({
      proRegTxHash: '954112bb018895896cfa3c3d00761a045fc16b22f2170c1fbb029a2936c68f16',
      confirmedHash: '1de71625dbc973e2377ebd7da4fe6f8a8eb8af8c5a99373e36151a4fbe9947cc',
      service: '192.168.65.2:20101',
      pubKeyOperator: '06a9789fab00deae1464ed80bda281fc833f85959b04201645e5fc25635e3e7ecda30d13d328b721af0809fca3bf3b63',
      votingAddress: 'yfLLjdEynGQBdoPcCDUNAxu6pksYGzXKA4',
      isValid: true,
    });
  });

  it('should return no documents if operatorReward = 0', async () => {
    const result = await handleUpdatedPubKeyOperator(
      masternodeEntry,
      previousMasternodeEntry,
      dataContractFixture,
      true,
    );

    expect(result).to.deep.equal({
      create: [],
      delete: [],
    });

    expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(
      masternodeEntry.proRegTxHash,
    );
    expect(stateRepositoryMock.fetchIdentity).to.be.not.called();
    expect(stateRepositoryMock.fetchDocuments).to.be.not.called();
    expect(createMasternodeIdentityMock).to.be.not.called();
  });

  it('should return documents if operatorReward > 0', async () => {
    rawTransactionFixture = '03000100018dee5838d18e38e62436fec18f4df9159e72af98b2bdff967ba9594962167d66000000006b483045022100e240a14a286fc575d5b7fd2359ebd94b44ddb6abe6e328a5a9055d73b30608a90220542071ba0ec1476bcf3e97091f5e3d73c424e2d11ac7c9471a43115112ebb4ec012103644d63815114a4ba0f2add003278ee6a8e13ce0283ab5bc61594cc4a75930475feffffff01a1949800000000001976a9143359e023b6d6f161bb2727f1ae9053a5b1eaedd988ac00000000fd120101000000000086842e2d813096e76cd8193bbdc90b7e829fa4266f5370aa246f9cfe9299d9f10000000000000000000000000000ffffc0a841024e85d095b3e04f3004bbb4a8567c765ed169afbc36418e4c8c144bd6c62640fe3ae295973d512f83f7f541525a5da3c91e77ec02ff4dcd214e7431b7d2cc28e420ebfeb612eed095b3e04f3004bbb4a8567c765ed169afbc364101001976a9143359e023b6d6f161bb2727f1ae9053a5b1eaedd988ac546183d65046070847ac8ce38e871d7135068423a23f6f79e8b2a10b15998add4120d5a944b6bb0b885389a6e9fbc1d430021cb7a9489b07b2538b8cca5f017ac9535a1cc4e9e575b7560b41ab7d9e01e835c7acf71c647fb1ac947f6bac5fbc8554';
    stateRepositoryMock.fetchTransaction.resolves({
      data: rawTransactionFixture,
      height: 626,
    });

    const result = await handleUpdatedPubKeyOperator(
      masternodeEntry,
      previousMasternodeEntry,
      dataContractFixture,
      false,
    );

    expect(result).to.deep.equal({
      create: [documentsFixture[0]],
      delete: [documentsFixture[1]],
    });

    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(Identifier.from('BWpTcdybaKcLLMXVErB8LJpEhp9XKDuaVsAjCxQkQXPd'));
    expect(createMasternodeIdentityMock).to.be.not.called();
    expect(transactionalDppMock.document.create).to.be.calledOnceWithExactly(
      dataContractFixture,
      Identifier.from('xmdKYeUsEU49sncsu76TmtufyqwP1By92RX4e48oRUW'),
      'masternodeRewardShares',
      {
        payToId: Identifier.from('BWpTcdybaKcLLMXVErB8LJpEhp9XKDuaVsAjCxQkQXPd'),
        percentage: 1,
      },
    );
    expect(stateRepositoryMock.fetchDocuments).to.be.calledOnceWithExactly(
      masternodeRewardSharesContractId,
      'masternodeRewardShares',
      {
        limit: 100,
        startAfter: undefined,
        where: [
          ['$ownerId', '==', Identifier.from('B3dJHVDWcjC7i8MGwJodgb87M6oj48niNsRW9F8aoVzV')],
          ['payToId', '==', Identifier.from('AcLbs82zFkMdN3uSurZePaZtgVKiXtme6ECsNXjZsA22')],
        ],
      },
    );
  });

  it('should create masternode Identity', async () => {
    rawTransactionFixture = '03000100018dee5838d18e38e62436fec18f4df9159e72af98b2bdff967ba9594962167d66000000006b483045022100e240a14a286fc575d5b7fd2359ebd94b44ddb6abe6e328a5a9055d73b30608a90220542071ba0ec1476bcf3e97091f5e3d73c424e2d11ac7c9471a43115112ebb4ec012103644d63815114a4ba0f2add003278ee6a8e13ce0283ab5bc61594cc4a75930475feffffff01a1949800000000001976a9143359e023b6d6f161bb2727f1ae9053a5b1eaedd988ac00000000fd120101000000000086842e2d813096e76cd8193bbdc90b7e829fa4266f5370aa246f9cfe9299d9f10000000000000000000000000000ffffc0a841024e85d095b3e04f3004bbb4a8567c765ed169afbc36418e4c8c144bd6c62640fe3ae295973d512f83f7f541525a5da3c91e77ec02ff4dcd214e7431b7d2cc28e420ebfeb612eed095b3e04f3004bbb4a8567c765ed169afbc364101001976a9143359e023b6d6f161bb2727f1ae9053a5b1eaedd988ac546183d65046070847ac8ce38e871d7135068423a23f6f79e8b2a10b15998add4120d5a944b6bb0b885389a6e9fbc1d430021cb7a9489b07b2538b8cca5f017ac9535a1cc4e9e575b7560b41ab7d9e01e835c7acf71c647fb1ac947f6bac5fbc8554';
    stateRepositoryMock.fetchTransaction.resolves({
      data: rawTransactionFixture,
      height: 626,
    });
    stateRepositoryMock.fetchIdentity.resolves(
      null,
    );

    const result = await handleUpdatedPubKeyOperator(
      masternodeEntry,
      previousMasternodeEntry,
      dataContractFixture,
      true,
    );

    expect(result).to.deep.equal({
      create: [documentsFixture[0]],
      delete: [documentsFixture[1]],
    });

    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWithExactly(Identifier.from('BWpTcdybaKcLLMXVErB8LJpEhp9XKDuaVsAjCxQkQXPd'));
    expect(createMasternodeIdentityMock).to.be.calledWithExactly(
      Identifier.from('BWpTcdybaKcLLMXVErB8LJpEhp9XKDuaVsAjCxQkQXPd'),
      Buffer.from(masternodeEntry.pubKeyOperator, 'hex'),
      IdentityPublicKey.TYPES.BLS12_381,
    );
    expect(transactionalDppMock.document.create).to.be.calledOnceWithExactly(
      dataContractFixture,
      Identifier.from('xmdKYeUsEU49sncsu76TmtufyqwP1By92RX4e48oRUW'),
      'masternodeRewardShares',
      {
        payToId: Identifier.from('BWpTcdybaKcLLMXVErB8LJpEhp9XKDuaVsAjCxQkQXPd'),
        percentage: 1,
      },
    );
    expect(stateRepositoryMock.fetchDocuments).to.be.calledOnceWithExactly(
      masternodeRewardSharesContractId,
      'masternodeRewardShares',
      {
        limit: 100,
        startAfter: undefined,
        where: [
          ['$ownerId', '==', Identifier.from('B3dJHVDWcjC7i8MGwJodgb87M6oj48niNsRW9F8aoVzV')],
          ['payToId', '==', Identifier.from('AcLbs82zFkMdN3uSurZePaZtgVKiXtme6ECsNXjZsA22')],
        ],
      },
    );
  });
});
