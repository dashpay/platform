const { mocha: { startIPFS } } = require('@dashevo/js-evo-services-ctl');

const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');

const STPacketIpfsRepository = require('../../../lib/storage/stPacket/STPacketIpfsRepository');

const getSTPacketsFixture = require('../../../lib/test/fixtures/getSTPacketsFixture');
const getStateTransitionsFixture = require('../../../lib/test/fixtures/getStateTransitionsFixture');

describe('StateTransition', () => {
  let dppMock;
  let stPacket;
  let stateTransition;
  let stPacketRepository;
  let ipfsApi;

  startIPFS().then((ipfs) => {
    ipfsApi = ipfs.getApi();
  });

  beforeEach(function beforeEach() {
    dppMock = createDPPMock(this.sinon);

    [stPacket] = getSTPacketsFixture();
    [stateTransition] = getStateTransitionsFixture();

    stPacketRepository = new STPacketIpfsRepository(
      ipfsApi,
      dppMock,
      1000,
    );
  });

  describe('#getPacketCID', () => {
    it('should create correct CID', async () => {
      stateTransition.extraPayload.setHashSTPacket(stPacket.hash());

      const cidFromIPFS = await stPacketRepository.store(stPacket);

      const cidFromST = stateTransition.getPacketCID();
      expect(cidFromST.toBaseEncodedString()).to.equal(cidFromIPFS.toBaseEncodedString());
    });
  });
});
