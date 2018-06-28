const startMongoDbInstance = require('../../../../lib/test/services/mocha/startMongoDbInstance');
const startIPFSInstance = require('../../../../lib/test/services/mocha/startIPFSInstance');
const getTransitionPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');
const getTransitionHeaderFixtures = require('../../../../lib/test/fixtures/getTransitionHeaderFixtures');
const StateTransitionHeader = require('../../../../lib/blockchain/StateTransitionHeader');
const DapContractMongoDbRepository = require('../../../../lib/stateView/dapContract/DapContractMongoDbRepository');
const storeDapContractFactory = require('../../../../lib/stateView/dapContract/storeDapContractFactory');
const hashSTPacket = require('../../../../lib/test/consensus/hashSTPacket');

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
    const packet = getTransitionPacketFixtures()[0];
    const header = getTransitionHeaderFixtures()[0].toJSON();
    header.hashSTPacket = await hashSTPacket(packet);

    const mongoClient = await mongoDbInstance.getMongoClient();
    const dapContractRepository = new DapContractMongoDbRepository(mongoClient);
    const storeDapContract = storeDapContractFactory(dapContractRepository, ipfsClient);
    const stHeader = new StateTransitionHeader(header);

    await ipfsClient.dag.put(packet, {
      format: 'dag-cbor',
      hashAlg: 'sha2-256',
    });

    const cid = stHeader.getPacketCID();
    await storeDapContract(cid);

    const dapId = packet.data.dapid;
    const dapContract = await dapContractRepository.find(dapId);

    expect(dapContract.getDapId()).to.equal(dapId);
    expect(dapContract.getDapName()).to.equal(packet.data.objects[0].data.dapname);
    expect(dapContract.getSchema()).to.deep.equal(packet.schema);
  });
});
