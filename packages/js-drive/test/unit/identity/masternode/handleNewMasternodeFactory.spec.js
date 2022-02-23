const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const SimplifiedMNListEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListEntry');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const handleNewMasternodeFactory = require('../../../../lib/identity/masternode/handleNewMasternodeFactory');

describe('handleNewMasternodeFactory', () => {
  let handleNewMasternode;
  let transactionalDppMock;
  let stateRepositoryMock;
  let createMasternodeIdentityMock;
  let documentsFixture;
  let rawTransactionFixture;
  let masternodeEntry;
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

    createMasternodeIdentityMock = this.sinon.stub();

    handleNewMasternode = handleNewMasternodeFactory(
      transactionalDppMock,
      stateRepositoryMock,
      createMasternodeIdentityMock,
    );

    masternodeEntry = new SimplifiedMNListEntry({
      proRegTxHash: '954112bb018895896cfa3c3d00761a045fc16b22f2170c1fbb029a2936c68f16',
      confirmedHash: '1de71625dbc973e2377ebd7da4fe6f8a8eb8af8c5a99373e36151a4fbe9947cc',
      service: '192.168.65.2:20101',
      pubKeyOperator: '8e4c8c144bd6c62640fe3ae295973d512f83f7f541525a5da3c91e77ec02ff4dcd214e7431b7d2cc28e420ebfeb612ee',
      votingAddress: 'yfLLjdEynGQBdoPcCDUNAxu6pksYGzXKA4',
      isValid: true,
    });
  });

  it('should return no documents if operatorReward = 0', async () => {
    const result = await handleNewMasternode(masternodeEntry, dataContractFixture, true);

    expect(result).to.deep.equal({
      create: [],
      delete: [],
    });

    expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWithExactly(
      masternodeEntry.proRegTxHash,
    );

    expect(createMasternodeIdentityMock).to.be.calledOnceWithExactly(
      Identifier.from('xmdKYeUsEU49sncsu76TmtufyqwP1By92RX4e48oRUW'),
      Buffer.from('4136bcaf69d15e767c56a8b4bb04304fe0b395d0', 'hex'),
      IdentityPublicKey.TYPES.ECDSA_HASH160,
    );

    // operatorReward = 0
    expect(transactionalDppMock.document.create).to.be.not.called();
  });

  it('should return a document if operatorReward > 0', async () => {
    // operatorReward = 1
    rawTransactionFixture = '03000100018dee5838d18e38e62436fec18f4df9159e72af98b2bdff967ba9594962167d66000000006b483045022100e240a14a286fc575d5b7fd2359ebd94b44ddb6abe6e328a5a9055d73b30608a90220542071ba0ec1476bcf3e97091f5e3d73c424e2d11ac7c9471a43115112ebb4ec012103644d63815114a4ba0f2add003278ee6a8e13ce0283ab5bc61594cc4a75930475feffffff01a1949800000000001976a9143359e023b6d6f161bb2727f1ae9053a5b1eaedd988ac00000000fd120101000000000086842e2d813096e76cd8193bbdc90b7e829fa4266f5370aa246f9cfe9299d9f10000000000000000000000000000ffffc0a841024e85d095b3e04f3004bbb4a8567c765ed169afbc36418e4c8c144bd6c62640fe3ae295973d512f83f7f541525a5da3c91e77ec02ff4dcd214e7431b7d2cc28e420ebfeb612eed095b3e04f3004bbb4a8567c765ed169afbc364101001976a9143359e023b6d6f161bb2727f1ae9053a5b1eaedd988ac546183d65046070847ac8ce38e871d7135068423a23f6f79e8b2a10b15998add4120d5a944b6bb0b885389a6e9fbc1d430021cb7a9489b07b2538b8cca5f017ac9535a1cc4e9e575b7560b41ab7d9e01e835c7acf71c647fb1ac947f6bac5fbc8554';
    stateRepositoryMock.fetchTransaction.resolves({
      data: rawTransactionFixture,
      height: 626,
    });

    const result = await handleNewMasternode(masternodeEntry, dataContractFixture, false);

    expect(result).to.deep.equal({
      create: [documentsFixture[0]],
      delete: [],
    });

    expect(transactionalDppMock.document.create).to.be.calledWithExactly(
      dataContractFixture,
      Identifier.from('xmdKYeUsEU49sncsu76TmtufyqwP1By92RX4e48oRUW'),
      'masternodeRewardShares',
      {
        payToId: Identifier.from('F1Fggqney3rdpLc69pS6CJr1yUxwEsRwmnBsLLCMjFsC'),
        percentage: 1,
      },
    );

    expect(createMasternodeIdentityMock).to.be.calledTwice();
    expect(createMasternodeIdentityMock.getCall(0)).to.be.calledWithExactly(
      Identifier.from('xmdKYeUsEU49sncsu76TmtufyqwP1By92RX4e48oRUW'),
      Buffer.from('4136bcaf69d15e767c56a8b4bb04304fe0b395d0', 'hex'),
      IdentityPublicKey.TYPES.ECDSA_HASH160,
    );
    expect(createMasternodeIdentityMock.getCall(1)).to.be.calledWithExactly(
      Identifier.from('F1Fggqney3rdpLc69pS6CJr1yUxwEsRwmnBsLLCMjFsC'),
      Buffer.from('8e4c8c144bd6c62640fe3ae295973d512f83f7f541525a5da3c91e77ec02ff4dcd214e7431b7d2cc28e420ebfeb612ee', 'hex'),
      IdentityPublicKey.TYPES.BLS12_381,
    );
  });
});
