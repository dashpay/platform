const { mocha: { startIPFS } } = require('js-evo-services-ctl');
const unpinAllIpfsPacketsFactory = require('../../../../lib/storage/ipfs/unpinAllIpfsPacketsFactory');

async function addPinPacket(ipfsApi) {
  const packet = {};
  const cid = await ipfsApi.dag.put(packet, { format: 'dag-cbor', hashAlg: 'sha2-256' });
  await ipfsApi.pin.add(cid.toBaseEncodedString(), { recursive: true });
  return cid.toBaseEncodedString();
}

const byCid = cid => object => object.hash === cid;

describe('unpinAllIpfsPacketsFactory', () => {
  let ipfsInstance;
  startIPFS().then((instance) => {
    ipfsInstance = instance;
  });

  it('should unpin all blocks in IPFS', async () => {
    const ipfsApi = ipfsInstance.getApi();
    const cid = await addPinPacket(ipfsApi);

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
