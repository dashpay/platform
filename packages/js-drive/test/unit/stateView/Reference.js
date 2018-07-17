const Reference = require('../../../lib/stateView/Reference');

describe('Reference', () => {
  it('should serialize Reference', () => {
    const blockHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const blockHeight = 1;
    const headerHash = '17jasdjk129uasd8asd023098SD09023jll123jlasd90823jklD';
    const hashSTPacket = 'ad877138as8012309asdkl123l123lka908013';
    const objectHash = '123981as90d01309ad09123';
    const reference = new Reference(
      blockHash,
      blockHeight,
      headerHash,
      hashSTPacket,
      objectHash,
    );

    const referenceSerialized = reference.toJSON();
    expect(referenceSerialized).to.deep.equal({
      blockHash,
      blockHeight,
      stHeaderHash: headerHash,
      stPacketHash: hashSTPacket,
      objectHash,
    });
  });
});
