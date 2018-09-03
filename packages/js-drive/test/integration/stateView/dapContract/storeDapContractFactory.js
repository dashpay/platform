const startMongoDbInstance = require('../../../../lib/test/services/mocha/startMongoDbInstance');
const startIPFSInstance = require('../../../../lib/test/services/mocha/startIPFSInstance');
const getTransitionPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');
const getTransitionHeaderFixtures = require('../../../../lib/test/fixtures/getTransitionHeaderFixtures');
const DapContractMongoDbRepository = require('../../../../lib/stateView/dapContract/DapContractMongoDbRepository');
const storeDapContractFactory = require('../../../../lib/stateView/dapContract/storeDapContractFactory');
const hashSTPacket = require('../../../../lib/test/consensus/hashSTPacket');
const sanitizeData = require('../../../../lib/mongoDb/sanitizeData');

describe('storeDapContractFactory', function main() {
  this.timeout(30000);

  let mongoDbInstance;
  startMongoDbInstance().then((_instance) => {
    mongoDbInstance = _instance;
  });

  let ipfsClient;
  startIPFSInstance().then((_instance) => {
    ipfsClient = _instance.getApi();
  });

  it('should store DAP schema', async () => {
    const packet = getTransitionPacketFixtures()[0].toJSON({ skipMeta: true });
    const header = getTransitionHeaderFixtures()[0];

    header.extraPayload.setHashSTPacket(await hashSTPacket(packet));

    const mongoClient = await mongoDbInstance.getMongoClient();
    const dapContractRepository = new DapContractMongoDbRepository(mongoClient, sanitizeData);
    const storeDapContract = storeDapContractFactory(dapContractRepository, ipfsClient);

    await ipfsClient.dag.put(packet, {
      format: 'dag-cbor',
      hashAlg: 'sha2-256',
    });

    const cid = header.getPacketCID();
    await storeDapContract(cid);

    const dapId = packet.dapid;
    const dapContract = await dapContractRepository.find(dapId);

    expect(dapContract.getDapId()).to.equal(dapId);
    expect(dapContract.getDapName()).to.equal(packet.dapcontract.dapname);
    expect(dapContract.getSchema()).to.deep.equal(packet.dapcontract.dapschema);
  });
});
