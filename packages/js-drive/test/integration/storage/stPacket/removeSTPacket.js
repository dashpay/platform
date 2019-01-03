const { mocha: { startIPFS } } = require('@dashevo/js-evo-services-ctl');

const StateTransitionPacket = require('../../../../lib/storage/stPacket/StateTransitionPacket');
const StateTransitionPacketIpfsRepository = require('../../../../lib/storage/stPacket/StateTransitionPacketIpfsRepository');
const removeSTPacketFactory = require('../../../../lib/storage/stPacket/removeSTPacketFactory');

const getPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');

describe('removeSTPacket', () => {
  let ipfsApi;
  startIPFS().then((_instance) => {
    ipfsApi = _instance.getApi();
  });

  let removeSTPacket;
  let stPacketRepository;
  beforeEach(() => {
    stPacketRepository = new StateTransitionPacketIpfsRepository(
      ipfsApi,
      1000,
    );
    removeSTPacket = removeSTPacketFactory(stPacketRepository);
  });

  it('should unpin previously stored and pinned packet', async () => {
    const [packetData] = getPacketFixtures();
    const packet = new StateTransitionPacket(packetData);

    const storedCid = await stPacketRepository.store(packet);
    await stPacketRepository.download(storedCid);

    await removeSTPacket(storedCid);

    let pinnedHashes = await ipfsApi.pin.ls();
    pinnedHashes = pinnedHashes.map(pin => pin.hash);

    expect(pinnedHashes).to.not.include(storedCid.toBaseEncodedString());
  });
});
