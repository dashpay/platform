const { mocha: { startMongoDb } } = require('@dashevo/dp-services-ctl');
const DashPlatformProtocol = require('@dashevo/dpp');

const SVContractMongoDbRepository = require('../../../../lib/stateView/contract/SVContractMongoDbRepository');
const fetchDPContractFactory = require('../../../../lib/stateView/contract/fetchDPContractFactory');

const getSVContractFixture = require('../../../../lib/test/fixtures/getSVContractFixture');

describe('fetchDPContractFactory', () => {
  let mongoDatabase;
  let svContractMongoDbRepository;
  let fetchDPContract;

  startMongoDb().then((mongoDb) => {
    mongoDatabase = mongoDb.getDb();
  });

  beforeEach(() => {
    const dpp = new DashPlatformProtocol();
    svContractMongoDbRepository = new SVContractMongoDbRepository(mongoDatabase, dpp);
    fetchDPContract = fetchDPContractFactory(svContractMongoDbRepository);
  });

  it('should return original DP contract data, if it is stored, by specific DP Contract id', async () => {
    const svContract = getSVContractFixture();

    await svContractMongoDbRepository.store(svContract);

    const contractData = await fetchDPContract(svContract.getContractId());

    expect(contractData.toJSON()).to.deep.equal(svContract.getDPContract().toJSON());
  });

  it('should return null if no DP contract were stored by specific DP Contract id', async () => {
    const contractId = 'c4970326400177ce67ec582425a698b85ae03cae2b0d168e87eed697f1388e4b';

    const contractData = await fetchDPContract(contractId);

    expect(contractData).to.be.null();
  });
});
