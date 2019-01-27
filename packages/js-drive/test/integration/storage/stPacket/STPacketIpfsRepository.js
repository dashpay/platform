const { mocha: { startIPFS } } = require('@dashevo/js-evo-services-ctl');

const DashPlatformProtocol = require('@dashevo/dpp');

const STPacketIpfsRepository = require('../../../../lib/storage/stPacket/STPacketIpfsRepository');

const getSTPacketsFixture = require('../../../../lib/test/fixtures/getSTPacketsFixture');

const createCIDFromHash = require('../../../../lib/storage/stPacket/createCIDFromHash');

const GetPacketTimeoutError = require('../../../../lib/storage/errors/GetPacketTimeoutError');
const PacketNotPinnedError = require('../../../../lib/storage/errors/PacketNotPinnedError');

describe('STPacketIpfsRepository', function main() {
  let stPacketRepository;
  let ipfsApi;
  let stPacket;

  this.timeout(60000);

  startIPFS().then((instance) => {
    ipfsApi = instance.getApi();
  });

  beforeEach(() => {
    const dataProviderMock = {};

    const dpp = new DashPlatformProtocol({
      dataProvider: dataProviderMock,
    });

    stPacketRepository = new STPacketIpfsRepository(
      ipfsApi,
      dpp,
      1000,
    );

    ([stPacket] = getSTPacketsFixture());
  });

  it('should store and find a packet', async () => {
    const storedCid = await stPacketRepository.store(stPacket);
    const storedPacket = await stPacketRepository.find(storedCid);

    expect(storedPacket.toJSON()).to.be.deep.equal(stPacket.toJSON());
  });

  it('should unpin previously stored and pinned packet', async () => {
    const storedCid = await stPacketRepository.store(stPacket);
    await stPacketRepository.download(storedCid);

    let pinnedHashes = await ipfsApi.pin.ls();
    pinnedHashes = pinnedHashes.map(pin => pin.hash);

    expect(pinnedHashes).to.include(storedCid.toBaseEncodedString());

    await stPacketRepository.delete(storedCid);

    pinnedHashes = await ipfsApi.pin.ls();
    pinnedHashes = pinnedHashes.map(pin => pin.hash);

    expect(pinnedHashes).to.not.include(storedCid.toBaseEncodedString());
  });

  it('should unpin all of the previously stored and pinned packets', async () => {
    const [stPacketOne, stPacketTwo] = getSTPacketsFixture();

    const storedCidOne = await stPacketRepository.store(stPacketOne);
    const storedCidTwo = await stPacketRepository.store(stPacketTwo);

    await stPacketRepository.download(storedCidOne);
    await stPacketRepository.download(storedCidTwo);

    let pinnedHashes = await ipfsApi.pin.ls();
    pinnedHashes = pinnedHashes.map(pin => pin.hash);

    expect(pinnedHashes).to.include(storedCidOne.toBaseEncodedString());
    expect(pinnedHashes).to.include(storedCidTwo.toBaseEncodedString());

    await stPacketRepository.deleteAll();

    pinnedHashes = await ipfsApi.pin.ls();
    pinnedHashes = pinnedHashes.map(pin => pin.hash);

    expect(pinnedHashes).to.not.include(storedCidOne.toBaseEncodedString());
    expect(pinnedHashes).to.not.include(storedCidTwo.toBaseEncodedString());
  });

  it('should not find a packet if it was not stored', async () => {
    const cid = createCIDFromHash(stPacket.hash());

    try {
      await stPacketRepository.find(cid);
      expect.fail('the error have not been thrown');
    } catch (e) {
      expect(e).to.be.instanceOf(GetPacketTimeoutError);
    }
  });

  it('should throw an error if trying to remove packet not pinned', async () => {
    const cid = createCIDFromHash(stPacket.hash());

    try {
      await stPacketRepository.delete(cid);
      expect.fail('the error have not been thrown');
    } catch (e) {
      expect(e).to.be.instanceOf(PacketNotPinnedError);
    }
  });
});
