const removeSTPacketMethodFactory = require('../../../../lib/api/methods/removeSTPacketMethodFactory');

const PacketNotPinnedError = require('../../../../lib/storage/errors/PacketNotPinnedError');
const InvalidHashError = require('../../../../lib/storage/stPacket/errors/InvalidHashError');
const InvalidParamsError = require('../../../../lib/api/InvalidParamsError');

describe('removeSTPacketMethodFactory', () => {
  let cid;
  let packetHash;
  let removeSTPacketMock;
  let removeSTPacketMethod;
  let createCIDFromHashMock;

  beforeEach(function beforeEach() {
    cid = {};
    packetHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';

    createCIDFromHashMock = this.sinon.stub().returns(cid);
    removeSTPacketMock = this.sinon.stub();

    removeSTPacketMethod = removeSTPacketMethodFactory(
      removeSTPacketMock,
      createCIDFromHashMock,
    );
  });

  it('should throw error if "packetHash" params is missing', async () => {
    let error;
    try {
      await removeSTPacketMethod({});
    } catch (e) {
      error = e;
    }

    expect(error).to.be.instanceOf(InvalidParamsError);

    expect(createCIDFromHashMock).not.to.be.called();
    expect(removeSTPacketMock).not.to.be.called();
  });

  it('should throw error if "packetHash" params is not a valid CID hash', async () => {
    packetHash = 'wrong';

    createCIDFromHashMock.throws(new InvalidHashError());

    let error;
    try {
      await removeSTPacketMethod({ packetHash });
    } catch (e) {
      error = e;
    }

    expect(error).to.be.instanceOf(InvalidParamsError);

    expect(createCIDFromHashMock).to.be.calledOnceWith(packetHash);
    expect(removeSTPacketMock).not.to.be.called();
  });

  it('should throw an original error if error is not InvalidHashError', async () => {
    const someError = new Error();

    createCIDFromHashMock.throws(someError);

    let error;
    try {
      await removeSTPacketMethod({ packetHash });
    } catch (e) {
      error = e;
    }

    expect(error).to.be.equal(someError);

    expect(createCIDFromHashMock).to.be.calledOnceWith(packetHash);
    expect(removeSTPacketMock).not.to.be.called();
  });

  it('should throw error if packet is not pinned', async () => {
    removeSTPacketMock.throws(new PacketNotPinnedError(cid));

    let error;
    try {
      await removeSTPacketMethod({ packetHash });
    } catch (e) {
      error = e;
    }

    expect(error).to.be.instanceOf(InvalidParamsError);

    expect(createCIDFromHashMock).to.be.calledOnceWith(packetHash);
    expect(removeSTPacketMock).to.be.calledOnceWith(cid);
  });

  it('should throw an original error if error is not PacketNotPinnedError', async () => {
    const otherError = new Error();

    removeSTPacketMock.throws(otherError);

    let error;
    try {
      await removeSTPacketMethod({ packetHash });
    } catch (e) {
      error = e;
    }

    expect(error).to.be.equal(otherError);

    expect(createCIDFromHashMock).to.be.calledOnceWith(packetHash);
    expect(removeSTPacketMock).to.be.calledOnceWith(cid);
  });

  it('should delete ST Packet', async () => {
    createCIDFromHashMock.returns(cid);

    await removeSTPacketMethod({ packetHash });

    expect(createCIDFromHashMock).to.be.calledOnceWith(packetHash);
    expect(removeSTPacketMock).to.be.calledOnceWith(cid);
  });
});
