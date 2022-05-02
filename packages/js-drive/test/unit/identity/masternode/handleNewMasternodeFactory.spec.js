const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const handleNewMasternodeFactory = require('../../../../lib/identity/masternode/handleNewMasternodeFactory');
const getSmlFixture = require('../../../../lib/test/fixtures/getSmlFixture');

describe('handleNewMasternodeFactory', () => {
  let handleNewMasternode;
  let dppMock;
  let stateRepositoryMock;
  let createMasternodeIdentityMock;
  let createRewardShareDocumentMock;
  let fetchTransactionMock;
  let transactionFixture;
  let masternodeEntry;
  let dataContract;

  beforeEach(function beforeEach() {
    const smlFixture = getSmlFixture();
    [masternodeEntry] = smlFixture[0].mnList;
    masternodeEntry.operatorPayoutAddress = 'yTCALGQTFNsA4pMPLTKAWdaLRmxfGpbujY';

    dataContract = getDataContractFixture();

    dppMock = createDPPMock(this.sinon);
    stateRepositoryMock = createStateRepositoryMock(this.sinon);

    createMasternodeIdentityMock = this.sinon.stub();
    createRewardShareDocumentMock = this.sinon.stub();

    transactionFixture = {
      extraPayload: {
        operatorReward: 0,
        keyIDOwner: Buffer.alloc(20).fill('a').toString('hex'),
      },
    };

    fetchTransactionMock = this.sinon.stub().resolves(transactionFixture);

    handleNewMasternode = handleNewMasternodeFactory(
      dppMock,
      stateRepositoryMock,
      createMasternodeIdentityMock,
      createRewardShareDocumentMock,
      fetchTransactionMock,
    );
  });

  it('should create masternode identity', async () => {
    await handleNewMasternode(masternodeEntry, dataContract);

    expect(fetchTransactionMock).to.be.calledOnceWithExactly(masternodeEntry.proRegTxHash);
    expect(createMasternodeIdentityMock).to.be.calledOnceWithExactly(
      Identifier.from('6k8jXHFuno3vqpfrQ36CaxrGi4SupdTJcGNeZLPioxQo'),
      Buffer.from('6161616161616161616161616161616161616161', 'hex'),
      IdentityPublicKey.TYPES.ECDSA_HASH160,
      Buffer.from('76a9144b682e8847992b8e9a2531cef5a3169ea5b80c5888ac', 'hex'),
    );
    expect(createRewardShareDocumentMock).to.not.be.called();
  });

  it('should create masternode identity and a document in rewards data contract with percentage', async () => {
    transactionFixture.extraPayload.operatorReward = 10;

    await handleNewMasternode(masternodeEntry, dataContract);

    expect(fetchTransactionMock).to.be.calledOnceWithExactly(masternodeEntry.proRegTxHash);
    expect(createMasternodeIdentityMock).to.be.calledTwice();
    expect(createMasternodeIdentityMock.getCall(0)).to.be.calledWith(
      Identifier.from('6k8jXHFuno3vqpfrQ36CaxrGi4SupdTJcGNeZLPioxQo'),
      Buffer.from('6161616161616161616161616161616161616161', 'hex'),
      IdentityPublicKey.TYPES.ECDSA_HASH160,
      Buffer.from('76a9144b682e8847992b8e9a2531cef5a3169ea5b80c5888ac', 'hex'),
    );
    expect(createMasternodeIdentityMock.getCall(1)).to.be.calledWith(
      Identifier.from('EwLi1FgGwvmLQ9nkfnttpXzv4SfC7XGBvs61QBCtnHEL'),
      Buffer.from('08b66151b81bd6a08bad2e68810ea07014012d6d804859219958a7fbc293689aa902bd0cd6db7a4699c9e88a4ae8c2c0', 'hex'),
      IdentityPublicKey.TYPES.BLS12_381,
      Buffer.from('76a9144b682e8847992b8e9a2531cef5a3169ea5b80c5888ac', 'hex'),
    );

    expect(createRewardShareDocumentMock).to.be.calledOnceWithExactly(
      dataContract,
      Identifier.from('6k8jXHFuno3vqpfrQ36CaxrGi4SupdTJcGNeZLPioxQo'),
      Identifier.from('EwLi1FgGwvmLQ9nkfnttpXzv4SfC7XGBvs61QBCtnHEL'),
      10,
    );
  });
});
