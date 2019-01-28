const { mocha: { startIPFS } } = require('@dashevo/js-evo-services-ctl');

const addSTPacketFactory = require('../../../lib/storage/stPacket/addSTPacketFactory');

const STPacketIpfsRepository = require('../../../lib/storage/stPacket/STPacketIpfsRepository');

const getSTPacketsFixture = require('../../../lib/test/fixtures/getSTPacketsFixture');
const getStateTransitionsFixture = require('../../../lib/test/fixtures/getStateTransitionsFixture');

const createDPPMock = require('../../../lib/test/mock/createDPPMock');

describe('StateTransition', () => {
  let dppMock;
  let stPacket;
  let stateTransition;
  let addSTPacket;
  let ipfsApi;

  startIPFS().then((instance) => {
    ipfsApi = instance.getApi();
  });

  beforeEach(function beforeEach() {
    dppMock = createDPPMock(this.sinon);

    ([stPacket] = getSTPacketsFixture());
    ([stateTransition] = getStateTransitionsFixture());

    const stPacketRepository = new STPacketIpfsRepository(
      ipfsApi,
      dppMock,
      1000,
    );

    addSTPacket = addSTPacketFactory(stPacketRepository);
  });

  it('should StateTransition packet CID equal to IPFS CID', async () => {
    stateTransition.extraPayload.setHashSTPacket(stPacket.hash());

    const ipfsCid = await addSTPacket(stPacket);

    const stPacketCid = stateTransition.getPacketCID();
    expect(stPacketCid.toBaseEncodedString()).to.equal(ipfsCid.toBaseEncodedString());
  });
});
