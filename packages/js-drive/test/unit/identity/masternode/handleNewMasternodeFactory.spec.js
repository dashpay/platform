const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

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
  let identityFixture;
  let Identifier;
  let KeyType;

  before(function before() {
    ({ Identifier, KeyType } = this.dppWasm);
  });

  beforeEach(function beforeEach() {
    const smlFixture = getSmlFixture();
    [masternodeEntry] = smlFixture[0].mnList;
    masternodeEntry.operatorPayoutAddress = 'yTCALGQTFNsA4pMPLTKAWdaLRmxfGpbujY';

    dataContract = getDataContractFixture();

    dppMock = createDPPMock(this.sinon);

    identityFixture = getIdentityFixture();

    createMasternodeIdentityMock = this.sinon.stub().resolves(identityFixture);
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
      this.dppWasm,
    );
  });

  it('should create masternode identity', async () => {
    masternodeEntry.payoutAddress = 'yRRwW957BJwL6SVVh3s8ASQYa2qXnduyfx';

    const payoutAddress = Address.fromString(masternodeEntry.payoutAddress);
    const payoutScript = new Script(payoutAddress);

    const result = await handleNewMasternode(masternodeEntry, dataContract, blockInfo);

    expect(result.createdEntities).to.have.lengthOf(2);
    expect(result.updatedEntities).to.have.lengthOf(0);
    expect(result.removedEntities).to.have.lengthOf(0);

    expect(result.createdEntities[0].toObject()).to.deep.equal(identityFixture.toObject());
    expect(result.createdEntities[1].toObject()).to.deep.equal(identityFixture.toObject());

    expect(fetchTransactionMock).to.be.calledOnceWithExactly(masternodeEntry.proRegTxHash);
    expect(createMasternodeIdentityMock.getCall(0)).to.be.calledWithExactly(
      blockInfo,
      Identifier.from('HYyu6DdUQyiHZwzeWpmahu7AUrsEF9MKkRcrdQnKeNSj'),
      Buffer.from('6161616161616161616161616161616161616161', 'hex'),
      KeyType.ECDSA_HASH160,
      payoutScript,
    );

    expect(createMasternodeIdentityMock.getCall(1)).to.be.calledWithExactly(
      blockInfo,
      Identifier.from('GVYoKVDd29gbmHzbVGepFjCbdymCS5Jq26CCiLnWNL6C'),
      Buffer.from('6262626262626262626262626262626262626262', 'hex'),
      KeyType.ECDSA_HASH160,
    );

    expect(createRewardShareDocumentMock).to.not.be.called();
  });

  it('should not create voting identity if keyIDVoting equals keyIDOwner', async () => {
    masternodeEntry.payoutAddress = 'yRRwW957BJwL6SVVh3s8ASQYa2qXnduyfx';

    const payoutAddress = Address.fromString(masternodeEntry.payoutAddress);
    const payoutScript = new Script(payoutAddress);

    transactionFixture.extraPayload.keyIDVoting = transactionFixture.extraPayload.keyIDOwner;

    fetchTransactionMock.resolves(transactionFixture);

    const result = await handleNewMasternode(masternodeEntry, dataContract, blockInfo);

    expect(result.createdEntities).to.have.lengthOf(1);
    expect(result.updatedEntities).to.have.lengthOf(0);
    expect(result.removedEntities).to.have.lengthOf(0);

    expect(result.createdEntities[0].toJSON()).to.deep.equal(identityFixture.toJSON());

    expect(fetchTransactionMock).to.be.calledOnceWithExactly(masternodeEntry.proRegTxHash);
    expect(createMasternodeIdentityMock).to.be.calledOnceWithExactly(
      blockInfo,
      Identifier.from('HYyu6DdUQyiHZwzeWpmahu7AUrsEF9MKkRcrdQnKeNSj'),
      Buffer.from('6161616161616161616161616161616161616161', 'hex'),
      KeyType.ECDSA_HASH160,
      payoutScript,
    );

    expect(createRewardShareDocumentMock).to.not.be.called();
  });

  it('should create masternode identity and a document in rewards data contract with percentage', async function test() {
    transactionFixture.extraPayload.operatorReward = 10;

    const result = await handleNewMasternode(masternodeEntry, dataContract, blockInfo);

    expect(result.createdEntities).to.have.lengthOf(3);
    expect(result.updatedEntities).to.have.lengthOf(0);
    expect(result.removedEntities).to.have.lengthOf(0);

    expect(result.createdEntities[0].toJSON()).to.deep.equal(identityFixture.toJSON());
    expect(result.createdEntities[1].toJSON()).to.deep.equal(identityFixture.toJSON());
    expect(result.createdEntities[2].toJSON()).to.deep.equal(identityFixture.toJSON());

    const operatorIdentifier = createOperatorIdentifier(this.dppWasm, masternodeEntry);
    const operatorPayoutAddress = Address.fromString(masternodeEntry.operatorPayoutAddress);
    const operatorPayoutScript = new Script(operatorPayoutAddress);

    const votingIdentifier = createVotingIdentifier(masternodeEntry, this.dppWasm);
    const payoutAddress = Address.fromString(masternodeEntry.payoutAddress);
    const payoutScript = new Script(payoutAddress);

    expect(fetchTransactionMock).to.be.calledOnceWithExactly(masternodeEntry.proRegTxHash);
    expect(createMasternodeIdentityMock).to.be.calledThrice();
    expect(createMasternodeIdentityMock.getCall(0)).to.be.calledWithExactly(
      blockInfo,
      Identifier.from('HYyu6DdUQyiHZwzeWpmahu7AUrsEF9MKkRcrdQnKeNSj'),
      Buffer.from('6161616161616161616161616161616161616161', 'hex'),
      KeyType.ECDSA_HASH160,
      payoutScript,
    );

    expect(createMasternodeIdentityMock.getCall(1)).to.be.calledWithExactly(
      blockInfo,
      operatorIdentifier,
      Buffer.from('951a3208ba531ea75aedd2dc0a9efc75f2c4d9492f1ee0a989b593bcd9722b1a101774d80a426552a9f91d24eb55af6e', 'hex'),
      KeyType.BLS12_381,
      operatorPayoutScript,
    );

    expect(createMasternodeIdentityMock.getCall(2)).to.be.calledWithExactly(
      blockInfo,
      votingIdentifier,
      Buffer.from('6262626262626262626262626262626262626262', 'hex'),
      KeyType.ECDSA_HASH160,
    );

    expect(createRewardShareDocumentMock).to.be.calledOnceWithExactly(
      dataContract,
      Identifier.from('HYyu6DdUQyiHZwzeWpmahu7AUrsEF9MKkRcrdQnKeNSj'),
      Identifier.from('4Ftw1Euv5BJrUk73gKeELFsVqrfVXjbUTSt4tNZjBaVq'),
      10,
      blockInfo,
    );
  });
});
