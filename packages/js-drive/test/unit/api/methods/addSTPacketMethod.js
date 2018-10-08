const cbor = require('cbor');
const fs = require('fs');
const path = require('path');

const addSTPacketMethodFactory = require('../../../../lib/api/methods/addSTPacketMethodFactory');
const InvalidParamsError = require('../../../../lib/api/InvalidParamsError');

const getTransitionPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');

describe('addSTPacketMethod', () => {
  let addSTPacket;
  let addSTPacketMethod;
  let cid;
  const packet = getTransitionPacketFixtures()[0];

  beforeEach(function beforeEach() {
    cid = packet.getCID();
    addSTPacket = this.sinon.stub().returns(cid);
    addSTPacketMethod = addSTPacketMethodFactory(addSTPacket);
  });

  it('should throw error if "packet" params is missing', () => {
    expect(addSTPacketMethod({ })).to.be.rejectedWith(InvalidParamsError);
    expect(addSTPacket).to.not.be.called();
  });
  it('should throw error if "packet" params is not a serialized ST Packet', () => {
    expect(addSTPacketMethod({ packet: 'shit' })).to.be.rejectedWith(InvalidParamsError);
    expect(addSTPacket).to.not.be.called();
  });
  it('should add ST Packet', async () => {
    // TODO: extract to separate method
    const packetsJSON = fs.readFileSync(path.join(__dirname, '/../../../fixtures/stateTransitionPackets.json'));
    const packetsData = JSON.parse(packetsJSON);

    const serializedPacket = cbor.encodeCanonical(packetsData[0]);

    const cidString = await addSTPacketMethod({ packet: serializedPacket.toString('hex') });

    expect(addSTPacket).to.be.calledOnce();
    expect(cidString).to.be.equal(cid.toBaseEncodedString());
  });
});
