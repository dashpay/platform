const { mocha: { startIPFS } } = require('@dashevo/js-evo-services-ctl');

const StateTransitionPacket = require('../../../../lib/storage/stPacket/StateTransitionPacket');
const StateTransitionPacketIpfsRepository = require('../../../../lib/storage/stPacket/StateTransitionPacketIpfsRepository');
const getPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');

describe('StateTransitionPacketIpfsRepository', function main() {
  this.timeout(60000);

  let ipfsApi;
  startIPFS().then((_instance) => {
    ipfsApi = _instance.getApi();
  });

  let stPacketRepository;
  beforeEach(() => {
    stPacketRepository = new StateTransitionPacketIpfsRepository(
      ipfsApi,
      1000,
    );
  });

  it('should store and find a packet', async () => {
    const [packetData] = getPacketFixtures();
    const packet = new StateTransitionPacket(packetData);

    const storedCid = await stPacketRepository.store(packet);
    const storedPacket = await stPacketRepository.find(storedCid);

    expect(storedPacket.toJSON({ skipMeta: true })).to.be
      .deep.equal(packet.toJSON({ skipMeta: true }));
  });

  it('should unpin previously stored and pinned packet', async () => {
    const [packetData] = getPacketFixtures();
    const packet = new StateTransitionPacket(packetData);

    const storedCid = await stPacketRepository.store(packet);
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
    const [packetDataOne, packetDataTwo] = getPacketFixtures();
    const packetOne = new StateTransitionPacket(packetDataOne);
    const packetTwo = new StateTransitionPacket(packetDataTwo);

    const storedCidOne = await stPacketRepository.store(packetOne);
    const storedCidTwo = await stPacketRepository.store(packetTwo);

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
    const packet = new StateTransitionPacket({
      pver: 42,
    });

    try {
      await stPacketRepository.find(packet.getCID());
      expect.fail('the error have not been thrown');
    } catch (e) {
      expect(e.name).to.be.equal('GetPacketTimeoutError');
    }
  });

  it('should throw an error if trying to remove packet not pinned', async () => {
    const packet = new StateTransitionPacket({
      pver: 42,
    });

    try {
      await stPacketRepository.delete(packet.getCID());
      expect.fail('the error have not been thrown');
    } catch (e) {
      expect(e.name).to.be.equal('PacketNotPinnedError');
    }
  });
});
