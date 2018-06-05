const DapContract = require('../../../../lib/stateView/dapContract/DapContract');
const DapContractMongoDbRepository = require('../../../../lib/stateView/dapContract/DapContractMongoDbRepository');
const startMongoDbInstance = require('../../../../lib/test/services/mocha/startMongoDbInstance');

describe('DapContractRepository', () => {
  let mongoDbInstance;
  startMongoDbInstance().then((_instance) => {
    mongoDbInstance = _instance;
  });

  it('should store DapContract entity', async () => {
    const dapId = '123456';
    const dapName = 'DashPay';
    const packetHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const schema = {};
    const dapContract = new DapContract(dapId, dapName, packetHash, schema);

    const mongoClient = await mongoDbInstance.getMongoClient();
    const dapContractRepository = new DapContractMongoDbRepository(mongoClient);
    await dapContractRepository.store(dapContract);
    const contract = await dapContractRepository.find(dapId);

    expect(contract.toJSON()).to.deep.equal(dapContract.toJSON());
  });
});
