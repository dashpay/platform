const StateTransitionPacket = require('../../../../lib/storage/stPacket/StateTransitionPacket');

const getTransitionPackets = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');

describe('StateTransitionPacket', () => {
  it('should create CID from a correct packet hash', async () => {
    const [packet] = await getTransitionPackets();
    const packetCIDString = 'zdrSLD5FaEb3A3ZcmVw99qsVaLQCxeLzowkwmuUwsz6sa5hSn';

    const cid = StateTransitionPacket.createCIDFromHash(packet.getHash());

    expect(cid.toBaseEncodedString()).to.be.equal(packetCIDString);
  });

  it('should throw InvalidHashError if packet hash is wrong', () => {
    try {
      StateTransitionPacket.createCIDFromHash('wrong');
      expect.fail('the error have not been thrown');
    } catch (e) {
      expect(e.name).to.be.equal('InvalidHashError');
    }
  });
});
