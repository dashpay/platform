const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Address = require('@dashevo/dashcore-lib/lib/address');
const Script = require('@dashevo/dashcore-lib/lib/script');
const handleNewMasternodeFactory = require('../../../../lib/identity/masternode/handleNewMasternodeFactory');
const getSmlFixture = require('../../../../lib/test/fixtures/getSmlFixture');
const createOperatorIdentifier = require('../../../../lib/identity/masternode/createOperatorIdentifier');
const BlockInfo = require('../../../../lib/blockExecution/BlockInfo');
const createVotingIdentifier = require('../../../../lib/identity/masternode/createVotingIdentifier');

describe('handleNewMasternodeFactory', () => {
  let handleNewMasternode;
  let dppMock;
  let createMasternodeIdentityMock;
  let createRewardShareDocumentMock;
  let fetchTransactionMock;
  let transactionFixture;
  let masternodeEntry;
  let dataContract;
  let blockInfo;

  beforeEach(function beforeEach() {
    const smlFixture = getSmlFixture();
    [masternodeEntry] = smlFixture[0].mnList;
    masternodeEntry.operatorPayoutAddress = 'yTCALGQTFNsA4pMPLTKAWdaLRmxfGpbujY';

    dataContract = getDataContractFixture();

    dppMock = createDPPMock(this.sinon);

    createMasternodeIdentityMock = this.sinon.stub();
    createRewardShareDocumentMock = this.sinon.stub();

    blockInfo = new BlockInfo(1, 0, Date.now());

    transactionFixture = {
      extraPayload: {
        operatorReward: 0,
        keyIDOwner: Buffer.alloc(20).fill('a').toString('hex'),
        keyIDVoting: Buffer.alloc(20).fill('b').toString('hex'),
      },
    };

    fetchTransactionMock = this.sinon.stub().resolves(transactionFixture);

    handleNewMasternode = handleNewMasternodeFactory(
      dppMock,
      createMasternodeIdentityMock,
      createRewardShareDocumentMock,
      fetchTransactionMock,
    );
  });

  it('should create masternode identity', async () => {
    masternodeEntry.payoutAddress = 'yRRwW957BJwL6SVVh3s8ASQYa2qXnduyfx';

    const payoutAddress = Address.fromString(masternodeEntry.payoutAddress);
    const payoutScript = new Script(payoutAddress);

    await handleNewMasternode(masternodeEntry, dataContract, blockInfo);

    expect(fetchTransactionMock).to.be.calledOnceWithExactly(masternodeEntry.proRegTxHash);
    expect(createMasternodeIdentityMock.getCall(0)).to.be.calledWithExactly(
      Identifier.from('6k8jXHFuno3vqpfrQ36CaxrGi4SupdTJcGNeZLPioxQo'),
      Buffer.from('6161616161616161616161616161616161616161', 'hex'),
      IdentityPublicKey.TYPES.ECDSA_HASH160,
      payoutScript,
    );

    expect(createMasternodeIdentityMock.getCall(1)).to.be.calledWithExactly(
      Identifier.from('G1p14MYdpNRLNWuKgQ9SjJUPxfuaJMTwYjdRWu9sLzvL'),
      Buffer.from('6262626262626262626262626262626262626262', 'hex'),
      IdentityPublicKey.TYPES.ECDSA_HASH160,
    );

    expect(createRewardShareDocumentMock).to.not.be.called();
  });

  it('should not create voting identity if keyIDVoting equals keyIDOwner', async () => {
    masternodeEntry.payoutAddress = 'yRRwW957BJwL6SVVh3s8ASQYa2qXnduyfx';

    const payoutAddress = Address.fromString(masternodeEntry.payoutAddress);
    const payoutScript = new Script(payoutAddress);

    transactionFixture.extraPayload.keyIDVoting = transactionFixture.extraPayload.keyIDOwner;

    fetchTransactionMock.resolves(transactionFixture);

    await handleNewMasternode(masternodeEntry, dataContract, blockInfo);

    expect(fetchTransactionMock).to.be.calledOnceWithExactly(masternodeEntry.proRegTxHash);
    expect(createMasternodeIdentityMock).to.be.calledOnceWithExactly(
      Identifier.from('6k8jXHFuno3vqpfrQ36CaxrGi4SupdTJcGNeZLPioxQo'),
      Buffer.from('6161616161616161616161616161616161616161', 'hex'),
      IdentityPublicKey.TYPES.ECDSA_HASH160,
      payoutScript,
    );

    expect(createRewardShareDocumentMock).to.not.be.called();
  });

  it('should create masternode identity and a document in rewards data contract with percentage', async () => {
    transactionFixture.extraPayload.operatorReward = 10;

    await handleNewMasternode(masternodeEntry, dataContract, blockInfo);

    const operatorIdentifier = createOperatorIdentifier(masternodeEntry);
    const operatorPayoutAddress = Address.fromString(masternodeEntry.operatorPayoutAddress);
    const operatorPayoutScript = new Script(operatorPayoutAddress);

    const votingIdentifier = createVotingIdentifier(masternodeEntry);

    expect(fetchTransactionMock).to.be.calledOnceWithExactly(masternodeEntry.proRegTxHash);
    expect(createMasternodeIdentityMock).to.be.calledThrice();
    expect(createMasternodeIdentityMock.getCall(0)).to.be.calledWith(
      Identifier.from('6k8jXHFuno3vqpfrQ36CaxrGi4SupdTJcGNeZLPioxQo'),
      Buffer.from('6161616161616161616161616161616161616161', 'hex'),
      IdentityPublicKey.TYPES.ECDSA_HASH160,
      undefined,
    );
    expect(createMasternodeIdentityMock.getCall(1)).to.be.calledWith(
      operatorIdentifier,
      Buffer.from('08b66151b81bd6a08bad2e68810ea07014012d6d804859219958a7fbc293689aa902bd0cd6db7a4699c9e88a4ae8c2c0', 'hex'),
      IdentityPublicKey.TYPES.BLS12_381,
      operatorPayoutScript,
    );
    expect(createMasternodeIdentityMock.getCall(2)).to.be.calledWith(
      votingIdentifier,
      Buffer.from('6262626262626262626262626262626262626262', 'hex'),
      IdentityPublicKey.TYPES.ECDSA_HASH160,
    );

    expect(createRewardShareDocumentMock).to.be.calledOnceWithExactly(
      dataContract,
      Identifier.from('6k8jXHFuno3vqpfrQ36CaxrGi4SupdTJcGNeZLPioxQo'),
      Identifier.from('EwLi1FgGwvmLQ9nkfnttpXzv4SfC7XGBvs61QBCtnHEL'),
      10,
      blockInfo,
    );
  });
});
