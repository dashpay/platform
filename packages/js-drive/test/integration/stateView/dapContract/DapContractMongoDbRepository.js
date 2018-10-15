const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');

const Reference = require('../../../../lib/stateView/Reference');
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
    const reference = new Reference();
    const schema = {};
    const version = 2;
    const previousVersions = [{
      version: 1,
      reference: new Reference(),
    }];
    const dapContract = new DapContract(
      dapId,
      dapName,
      reference,
      schema,
      version,
      previousVersions,
    );

    await dapContractRepository.store(dapContract);
    const contract = await dapContractRepository.find(dapId);
    expect(contract.toJSON()).to.deep.equal(dapContract.toJSON());
  });

  it('should return null if not found', async () => {
    const contract = await dapContractRepository.find('unknown');

    expect(contract).to.be.null();
  });
});
