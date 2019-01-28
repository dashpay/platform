const { mocha: { startIPFS } } = require('@dashevo/js-evo-services-ctl');

const DashPlatformProtocol = require('@dashevo/dpp');

const STPacketIpfsRepository = require('../../../../lib/storage/stPacket/STPacketIpfsRepository');

const removeSTPacketFactory = require('../../../../lib/storage/stPacket/removeSTPacketFactory');

const getSTPacketsFixture = require('../../../../lib/test/fixtures/getSTPacketsFixture');

describe('removeSTPacket', () => {
  let ipfsApi;
  let removeSTPacket;
  let stPacketRepository;

  startIPFS().then((ipfs) => {
    ipfsApi = ipfs.getApi();
  });

  beforeEach(() => {
    const dpp = new DashPlatformProtocol();

    stPacketRepository = new STPacketIpfsRepository(
      ipfsApi,
      dpp,
      1000,
    );

    removeSTPacket = removeSTPacketFactory(stPacketRepository);
  });

  it('should unpin previously stored and pinned packet', async () => {
    const [stPacket] = getSTPacketsFixture();

    const storedCid = await stPacketRepository.store(stPacket);

    await stPacketRepository.download(storedCid);

    await removeSTPacket(storedCid);

    let pinnedHashes = await ipfsApi.pin.ls();
    pinnedHashes = pinnedHashes.map(pin => pin.hash);

    expect(pinnedHashes).to.not.include(storedCid.toBaseEncodedString());
  });
});
