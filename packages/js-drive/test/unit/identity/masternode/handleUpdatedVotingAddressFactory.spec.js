const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const handleUpdatedVotingAddressFactory = require('../../../../lib/identity/masternode/handleUpdatedVotingAddressFactory');
const StorageResult = require('../../../../lib/storage/StorageResult');
const BlockInfo = require('../../../../lib/blockExecution/BlockInfo');

describe('handleUpdatedVotingAddressFactory', () => {
  let handleUpdatedVotingAddress;
  let createMasternodeIdentityMock;
  let smlEntry;
  let identity;
  let fetchTransactionMock;
  let transactionFixture;
  let identityRepositoryMock;
  let blockInfo;

  beforeEach(function beforeEach() {
    smlEntry = {
      proRegTxHash: '5557273f5922d9925e2327908ddb128bcf8e055a04d86e23431809bedd077060',
      confirmedHash: '0000003da09fd100c60ad5743c44257bb9220ad8162a9b6cae9d005c8e465dba',
      service: '95.222.25.60:19997',
      pubKeyOperator: '08b66151b81bd6a08bad2e68810ea07014012d6d804859219958a7fbc293689aa902bd0cd6db7a4699c9e88a4ae8c2c0',
      votingAddress: 'yZRteAQ51BoeD3sJL1iGdt6HJLgkWGurw5',
      isValid: false,
    };

    transactionFixture = {
      extraPayload: {
        operatorReward: 0,
        keyIDOwner: Buffer.alloc(20).fill('a').toString('hex'),
        keyIDVoting: Buffer.alloc(20).fill('b').toString('hex'),
      },
    };

    fetchTransactionMock = this.sinon.stub().resolves(transactionFixture);

    identity = getIdentityFixture();

    identityRepositoryMock = {
      update: this.sinon.stub(),
      fetch: this.sinon.stub().resolves(new StorageResult(null, [])),
    };

    createMasternodeIdentityMock = this.sinon.stub();

    blockInfo = new BlockInfo(1, 0, Date.now());

    handleUpdatedVotingAddress = handleUpdatedVotingAddressFactory(
      identityRepositoryMock,
      createMasternodeIdentityMock,
      fetchTransactionMock,
    );
  });

  it('should store updated identity', async () => {
    createMasternodeIdentityMock.resolves(identity);

    const result = await handleUpdatedVotingAddress(smlEntry, blockInfo);

    expect(result.createdEntities).to.have.lengthOf(1);
    expect(result.updatedEntities).to.have.lengthOf(0);
    expect(result.removedEntities).to.have.lengthOf(0);

    expect(result.createdEntities[0]).to.deep.equal(identity);
    expect(createMasternodeIdentityMock).to.be.calledOnceWithExactly(
      blockInfo,
      Identifier.from('G1p14MYdpNRLNWuKgQ9SjJUPxfuaJMTwYjdRWu9sLzvL'),
      Buffer.from('8fd1a9502c58ab103792693e951bf39f10ee46a9', 'hex'),
      IdentityPublicKey.TYPES.ECDSA_HASH160,
    );
    expect(fetchTransactionMock).to.be.calledWithExactly(smlEntry.proRegTxHash);
  });

  it('should not update identity if identity already exists', async () => {
    identityRepositoryMock.fetch.resolves(new StorageResult(identity, []));

    const result = await handleUpdatedVotingAddress(smlEntry, blockInfo);

    expect(result.createdEntities).to.have.lengthOf(0);
    expect(result.updatedEntities).to.have.lengthOf(0);
    expect(result.removedEntities).to.have.lengthOf(0);

    expect(createMasternodeIdentityMock).to.not.be.called();
    expect(fetchTransactionMock).to.be.calledWithExactly(smlEntry.proRegTxHash);
  });

  it('should not update identity if owner and voting keys are the same', async () => {
    transactionFixture.extraPayload.keyIDVoting = transactionFixture.extraPayload.keyIDOwner;

    fetchTransactionMock.resolves(transactionFixture);

    const result = await handleUpdatedVotingAddress(smlEntry, blockInfo);

    expect(result.createdEntities).to.have.lengthOf(0);
    expect(result.updatedEntities).to.have.lengthOf(0);
    expect(result.removedEntities).to.have.lengthOf(0);

    expect(createMasternodeIdentityMock).to.not.be.called();
    expect(fetchTransactionMock).to.be.calledWithExactly(smlEntry.proRegTxHash);
  });
});
