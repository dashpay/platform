const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const handleUpdatedVotingAddressFactory = require('../../../../lib/identity/masternode/handleUpdatedVotingAddressFactory');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

describe('handleUpdatedVotingAddressFactory', () => {
  let handleUpdatedVotingAddress;
  let createMasternodeIdentityMock;
  let stateRepositoryMock;
  let smlEntry;
  let identity;

  beforeEach(function beforeEach() {
    smlEntry = {
      proRegTxHash: '5557273f5922d9925e2327908ddb128bcf8e055a04d86e23431809bedd077060',
      confirmedHash: '0000003da09fd100c60ad5743c44257bb9220ad8162a9b6cae9d005c8e465dba',
      service: '95.222.25.60:19997',
      pubKeyOperator: '08b66151b81bd6a08bad2e68810ea07014012d6d804859219958a7fbc293689aa902bd0cd6db7a4699c9e88a4ae8c2c0',
      votingAddress: 'yZRteAQ51BoeD3sJL1iGdt6HJLgkWGurw5',
      isValid: false,
    };

    identity = getIdentityFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    createMasternodeIdentityMock = this.sinon.stub();

    stateRepositoryMock.fetchIdentity.resolves(null);

    handleUpdatedVotingAddress = handleUpdatedVotingAddressFactory(
      stateRepositoryMock,
      createMasternodeIdentityMock,
    );
  });

  it('should store updated identity', async () => {
    createMasternodeIdentityMock.resolves(identity);

    const result = await handleUpdatedVotingAddress(smlEntry);

    expect(result).to.have.lengthOf(1);
    expect(result[0]).to.deep.equal(identity);
    expect(createMasternodeIdentityMock).to.be.calledOnceWithExactly(
      Identifier.from('G1p14MYdpNRLNWuKgQ9SjJUPxfuaJMTwYjdRWu9sLzvL'),
      Buffer.from('8fd1a9502c58ab103792693e951bf39f10ee46a9', 'hex'),
      IdentityPublicKey.TYPES.ECDSA_HASH160,
    );
  });

  it('should not update identity if identity already exists', async () => {
    stateRepositoryMock.fetchIdentity.resolves(identity);

    const result = await handleUpdatedVotingAddress(smlEntry);

    expect(result).to.have.lengthOf(0);
    expect(createMasternodeIdentityMock).to.not.be.called();
  });
});
