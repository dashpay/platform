const { mocha: { startIPFS } } = require('@dashevo/dp-services-ctl');

const DashPlatformProtocol = require('@dashevo/dpp');

const createDataProviderMock = require('@dashevo/dpp/lib/test/mocks/createDataProviderMock');

const STPacketIpfsRepository = require('../../../../lib/storage/stPacket/STPacketIpfsRepository');

const addSTPacketFactory = require('../../../../lib/storage/stPacket/addSTPacketFactory');

const getSTPacketsFixture = require('../../../../lib/test/fixtures/getSTPacketsFixture');
const getStateTransitionsFixture = require('../../../../lib/test/fixtures/getStateTransitionsFixture');

const InvalidSTPacketDataError = require('../../../../lib/storage/stPacket/errors/InvalidSTPacketDataError');

describe('addSTPacketFactory', () => {
  let ipfsApi;
  let addSTPacket;
  let stPacket;
  let stateTransition;
  let stPacketRepository;
  let dataProviderMock;

  startIPFS().then((ipfs) => {
    ipfsApi = ipfs.getApi();
  });

  beforeEach(function beforeEach() {
    [stPacket] = getSTPacketsFixture();
    [stateTransition] = getStateTransitionsFixture();

    stateTransition.extraPayload.hashSTPacket = stPacket.hash();

    dataProviderMock = createDataProviderMock(this.sinon);

    const dpp = new DashPlatformProtocol({
      dataProvider: dataProviderMock,
    });

    stPacketRepository = new STPacketIpfsRepository(
      ipfsApi,
      dpp,
      1000,
    );

    addSTPacket = addSTPacketFactory(stPacketRepository, dpp);
  });

  it('should throw an error if ST or ST Packet is invalid', async () => {
    try {
      await addSTPacket(stPacket, stateTransition);
      expect.fail('should throw InvalidSTPacketDataError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidSTPacketDataError);
      expect(e.getErrors()).to.lengthOf(1);

      const [consensusError] = e.getErrors();

      expect(consensusError.name).to.equal('UserNotFoundError');
    }
  });

  it('should add ST Packet to storage', async () => {
    dataProviderMock.fetchTransaction.resolves({
      confirmations: 6,
    });

    const stPacketCID = await addSTPacket(stPacket, stateTransition);

    const stPacketFromIPFS = await stPacketRepository.find(stPacketCID);

    expect(stPacketFromIPFS.toJSON()).to.deep.equal(stPacket.toJSON());
  });
});
