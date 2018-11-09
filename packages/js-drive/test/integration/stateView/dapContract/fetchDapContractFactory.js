const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');

const Reference = require('../../../../lib/stateView/Reference');
const DapContract = require('../../../../lib/stateView/dapContract/DapContract');
const DapContractMongoDbRepository = require('../../../../lib/stateView/dapContract/DapContractMongoDbRepository');
const fetchDapContractFactory = require('../../../../lib/stateView/dapContract/fetchDapContractFactory');

const getPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');

const serializer = require('../../../../lib/util/serializer');

describe('fetchDapContractFactory', () => {
  let mongoDb;
  startMongoDb().then(async (instance) => {
    mongoDb = await instance.getDb();
  });

  let dapContractMongoDbRepository;
  let fetchDapContract;
  beforeEach(() => {
    dapContractMongoDbRepository = new DapContractMongoDbRepository(mongoDb, serializer);
    fetchDapContract = fetchDapContractFactory(dapContractMongoDbRepository);
  });

  it('should return original DAP contract data, if it is stored, by specific DAP id', async () => {
    const dapId = '1234';
    const [packet] = getPacketFixtures();
    const dapContract = new DapContract(
      dapId,
      packet.dapcontract,
      new Reference(),
      false,
    );

    await dapContractMongoDbRepository.store(dapContract);

    const contractData = await fetchDapContract(dapId);

    expect(contractData).to.be.deep.equal(dapContract.getOriginalData());
  });

  it('should return null if no DAP contract were stored by specific DAP id', async () => {
    const contractData = await fetchDapContract('1234');
    expect(contractData).to.be.null();
  });
});
