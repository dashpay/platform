const proxyquire = require('proxyquire');

const PacketNotPinnedError = require('../../../../lib/storage/errors/PacketNotPinnedError');
const InvalidHashError = require('../../../../lib/storage/errors/InvalidHashError');
const InvalidParamsError = require('../../../../lib/api/InvalidParamsError');

describe('removeSTPacketMethod', () => {
  let removeSTPacket;
  let removeSTPacketMethod;
  let stateTransitonPacketMock;
  beforeEach(function beforeEach() {
    stateTransitonPacketMock = {
      createCIDFromHash: this.sinon.stub(),
    };

    const removeSTPacketMethodFactory = proxyquire(
      '../../../../lib/api/methods/removeSTPacketMethodFactory',
      {
        '../../storage/stPacket/StateTransitionPacket': stateTransitonPacketMock,
      },
    );

    removeSTPacket = this.sinon.stub();
    removeSTPacketMethod = removeSTPacketMethodFactory(removeSTPacket);
  });

  it('should throw error if "packetHash" params is missing', () => {
    expect(removeSTPacketMethod({ })).to.be.rejectedWith(InvalidParamsError);
    expect(removeSTPacket).to.not.be.called();
  });

  it('should throw error if "packetHash" params is not a valid CID hash', () => {
    stateTransitonPacketMock.createCIDFromHash.throws(new InvalidHashError());
    expect(removeSTPacketMethod({ packetHash: 'wrong' })).to.be.rejectedWith(InvalidParamsError);
    expect(removeSTPacket).to.not.be.called();
  });

  it('should throw an original error if error is not InvalidHashError', () => {
    const error = new Error('Some unknown error');
    stateTransitonPacketMock.createCIDFromHash.throws(error);
    expect(removeSTPacketMethod({ packetHash: 'other' })).to.be.rejectedWith(error);
  });

  it('should throw error if packet is not pinned', () => {
    stateTransitonPacketMock.createCIDFromHash.returns({});
    removeSTPacket.throws(new PacketNotPinnedError());
    expect(removeSTPacketMethod({ packetHash: 'non_existent' })).to.be.rejectedWith(InvalidParamsError);
  });

  it('should throw an original error if error is not PacketNotPinnedError', () => {
    const error = new Error('Some unknown error');
    stateTransitonPacketMock.createCIDFromHash.returns({});
    removeSTPacket.throws(error);
    expect(removeSTPacketMethod({ packetHash: 'other' })).to.be.rejectedWith(error);
  });

  it('should delete ST Packet', async () => {
    const cid = {
      value: 'some_potential_cid',
    };
    stateTransitonPacketMock.createCIDFromHash.returns(cid);

    await removeSTPacketMethod({ packetHash: 'some_packet_hash' });

    expect(removeSTPacket).to.be.calledOnceWith(cid);
  });
});
