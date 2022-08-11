const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const createVotingIdentifier = require('../../../../lib/identity/masternode/createVotingIdentifier');

describe('createVotingIdentifier', () => {
  let smlEntry;

  beforeEach(() => {
    smlEntry = {
      proRegTxHash: '5557273f5922d9925e2327908ddb128bcf8e055a04d86e23431809bedd077060',
      confirmedHash: '0000003da09fd100c60ad5743c44257bb9220ad8162a9b6cae9d005c8e465dba',
      service: '95.222.25.60:19997',
      pubKeyOperator: '08b66151b81bd6a08bad2e68810ea07014012d6d804859219958a7fbc293689aa902bd0cd6db7a4699c9e88a4ae8c2c0',
      votingAddress: 'yZRteAQ51BoeD3sJL1iGdt6HJLgkWGurw5',
      isValid: false,
    };
  });

  it('should return voting identifier from smlEntry', () => {
    const identifier = createVotingIdentifier(smlEntry);

    expect(identifier).to.deep.equal(Identifier.from('G1p14MYdpNRLNWuKgQ9SjJUPxfuaJMTwYjdRWu9sLzvL'));
  });
});
