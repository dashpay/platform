const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');
const DapContract = require('../../../../lib/stateView/dapContract/DapContract');
const DapContractMongoDbRepository = require('../../../../lib/stateView/dapContract/DapContractMongoDbRepository');
const sanitizeData = require('../../../../lib/mongoDb/sanitizeData');

describe('DapContractRepository', () => {
  let dapContractRepository;
  startMongoDb().then(async (mongoDbInstance) => {
    const mongoDb = await mongoDbInstance.getDb();
    dapContractRepository = new DapContractMongoDbRepository(mongoDb, sanitizeData);
  });

  it('should store DapContract entity', async () => {
    const dapId = '123456';
    const dapName = 'DashPay';
    const packetHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    const schema = {};
    const dapContract = new DapContract(dapId, dapName, packetHash, schema);

    await dapContractRepository.store(dapContract);
    const contract = await dapContractRepository.find(dapId);
    expect(contract.toJSON()).to.deep.equal(dapContract.toJSON());
  });

  it('should return null if not found', async () => {
    const contract = await dapContractRepository.find('unknown');

    expect(contract).to.be.null();
  });
});
