const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');
const Reference = require('../../../../lib/stateView/Reference');
const DapContract = require('../../../../lib/stateView/dapContract/DapContract');
const DapContractMongoDbRepository = require('../../../../lib/stateView/dapContract/DapContractMongoDbRepository');
const serializer = require('../../../../lib/util/serializer');

describe('DapContractRepository', () => {
  let dapContractRepository;
  startMongoDb().then(async (mongoDbInstance) => {
    const mongoDb = await mongoDbInstance.getDb();
    dapContractRepository = new DapContractMongoDbRepository(mongoDb, serializer);
  });

  let dapContract;

  it('should store DapContract entity', async () => {
    const dapId = '123456';
    const dapName = 'DashPay';
    const reference = new Reference(
      null, null, 'someSTHeaderHash', null, null,
    );
    const version = 2;
    const data = {
      dapname: dapName,
      dapver: version,
      dapschema: {
        $a: 1,
        $b: {
          value: 'some',
        },
      },
      upgradedapid: '123',
    };
    const deleted = false;
    const previousVersions = [{
      version: 1,
      reference: new Reference(null, null, null, null, null),
    }];
    dapContract = new DapContract(
      dapId,
      data,
      reference,
      deleted,
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

  it('should find all contracts by stHeaderHash', async () => {
    await dapContractRepository.store(dapContract);
    const dapContracts = await dapContractRepository.findAllByReferenceSTHeaderHash(
      dapContract.reference.stHeaderHash,
    );
    expect(dapContracts.length).to.be.equal(1);
    expect(dapContracts).to.be.deep.equal([dapContract]);
  });

  it('should return null if contract was marked as deleted', async () => {
    dapContract.markAsDeleted();
    await dapContractRepository.store(dapContract);
    const contract = await dapContractRepository.find(dapContract.getDapId());
    expect(contract).to.be.null();
  });
});
