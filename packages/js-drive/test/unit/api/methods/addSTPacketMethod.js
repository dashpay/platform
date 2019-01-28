const serializer = require('@dashevo/dpp/lib/util/serializer');

const InvalidSTPacketError = require('@dashevo/dpp/lib/stPacket/errors/InvalidSTPacketError');

const addSTPacketMethodFactory = require('../../../../lib/api/methods/addSTPacketMethodFactory');
const createCIDFromHash = require('../../../../lib/storage/stPacket/createCIDFromHash');

const createDPPMock = require('../../../../lib/test/mock/createDPPMock');

const getSTPacketsFixture = require('../../../../lib/test/fixtures/getSTPacketsFixture');

const InvalidParamsError = require('../../../../lib/api/InvalidParamsError');

describe('addSTPacketMethod', () => {
  let stPacket;
  let cid;
  let dppMock;
  let addSTPacketMock;
  let addSTPacketMethod;

  beforeEach(function beforeEach() {
    [stPacket] = getSTPacketsFixture();

    cid = createCIDFromHash(stPacket.hash());

    dppMock = createDPPMock(this.sinon);

    addSTPacketMock = this.sinon.stub().resolves(cid);

    addSTPacketMethod = addSTPacketMethodFactory(addSTPacketMock, dppMock);
  });

  it('should throw error if "packet" params is missing', async () => {
    let error;
    try {
      await addSTPacketMethod({});
    } catch (e) {
      error = e;
    }

    expect(error).to.be.instanceOf(InvalidParamsError);

    expect(addSTPacketMock).to.not.be.called();
  });

  it('should throw error if "packet" params is not a serialized ST Packet', async () => {
    const wrongString = 'something';

    const cborError = new Error();

    dppMock.packet.createFromSerialized.throws(cborError);

    let error;
    try {
      await addSTPacketMethod({ packet: wrongString });
    } catch (e) {
      error = e;
    }

    expect(error).to.be.equal(cborError);

    expect(dppMock.packet.createFromSerialized).to.be.calledOnceWith(wrongString);

    expect(addSTPacketMock).to.not.be.called();
  });

  it('should throw error if "packet" params is not valid ST Packet', async () => {
    const invalidSTPacket = { ...stPacket.toJSON(), wrongField: true };

    const serializedSTPacket = serializer.encode(invalidSTPacket);

    const validationError = new InvalidSTPacketError([], invalidSTPacket);

    dppMock.packet.createFromSerialized.throws(validationError);

    let error;
    try {
      await addSTPacketMethod({ packet: serializedSTPacket });
    } catch (e) {
      error = e;
    }

    expect(error).to.be.instanceOf(InvalidParamsError);

    expect(dppMock.packet.createFromSerialized).to.be.calledOnceWith(serializedSTPacket);

    expect(addSTPacketMock).to.not.be.called();
  });

  it('should add ST Packet', async () => {
    const serializedSTPacket = stPacket.serialize().toString('hex');

    dppMock.packet.createFromSerialized.resolves(stPacket);

    const result = await addSTPacketMethod(
      { packet: serializedSTPacket },
    );

    expect(result).to.be.equal(cid.toBaseEncodedString());

    expect(dppMock.packet.createFromSerialized).to.be.calledOnceWith(serializedSTPacket);

    expect(addSTPacketMock).to.be.calledOnceWith(stPacket);
  });
});
