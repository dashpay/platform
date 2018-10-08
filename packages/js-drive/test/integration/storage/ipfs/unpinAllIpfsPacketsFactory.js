const { mocha: { startIPFS } } = require('@dashevo/js-evo-services-ctl');
const unpinAllIpfsPacketsFactory = require('../../../../lib/storage/ipfs/unpinAllIpfsPacketsFactory');
const getTransitionPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');
const addSTPacketFactory = require('../../../../lib/storage/ipfs/addSTPacketFactory');

const byCid = cid => object => object.hash === cid.toBaseEncodedString();

describe('unpinAllIpfsPacketsFactory', () => {
  let ipfsInstance;
  let addSTPacket;

  startIPFS().then((instance) => {
    ipfsInstance = instance;
  });

  beforeEach(() => {
    addSTPacket = addSTPacketFactory(ipfsInstance.getApi());
  });

  it('should unpin all blocks in IPFS', async () => {
    const packet = getTransitionPacketFixtures()[0];

    const ipfsApi = ipfsInstance.getApi();

    const cid = await addSTPacket(packet);
    await ipfsApi.pin.add(cid.toBaseEncodedString(), { recursive: true });

    const pinsetBefore = await ipfsApi.pin.ls();
    const filterBefore = pinsetBefore.filter(byCid(cid));
    expect(filterBefore.length).to.equal(1);

    const unpinAllPackets = unpinAllIpfsPacketsFactory(ipfsApi);
    await unpinAllPackets();

    const pinsetAfter = await ipfsApi.pin.ls();
    const filterAfter = pinsetAfter.filter(byCid(cid));
    expect(filterAfter.length).to.equal(0);
  });
});
