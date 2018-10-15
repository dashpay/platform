const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');

const getTransitionPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');
const doubleSha256 = require('../../../../lib/util/doubleSha256');
const Reference = require('../../../../lib/stateView/Reference');
const DapContract = require('../../../../lib/stateView/dapContract/DapContract');
const sanitizeData = require('../../../../lib/mongoDb/sanitizeData');
const DapContractMongoDbRepository = require('../../../../lib/stateView/dapContract/DapContractMongoDbRepository');
const updateDapContractFactory = require('../../../../lib/stateView/dapContract/updateDapContractFactory');

describe('updateDapContractFactory', () => {
  const blockHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
  const blockHeight = 1;
  const headerHash = '17jasdjk129uasd8asd023098SD09023jll123jlasd90823jklD';
  const hashSTPacket = 'ad877138as8012309asdkl123l123lka908013';
  const objectHash = '123981as90d01309ad09123';

  let mongoDb;
  startMongoDb().then(async (mongoDbInstance) => {
    mongoDb = await mongoDbInstance.getDb();
  });

  let dapContractRepository;
  let updateDapContract;
  beforeEach(() => {
    dapContractRepository = new DapContractMongoDbRepository(mongoDb, sanitizeData);
    updateDapContract = updateDapContractFactory(dapContractRepository);
  });

  it('should store DapContract', async () => {
    const packet = getTransitionPacketFixtures()[0];
    const dapContractData = packet.dapcontract;
    const dapId = doubleSha256(dapContractData);
    const reference = new Reference(
      blockHash,
      blockHeight,
      headerHash,
      hashSTPacket,
      objectHash,
    );

    await updateDapContract(dapId, reference, dapContractData);

    const dapContract = await dapContractRepository.find(dapId);
    expect(dapContract.getSchema()).to.deep.equal(dapContractData.dapschema);
    expect(dapContract.getVersion()).to.deep.equal(dapContractData.dapver);
  });

  it('should maintain DapContract previous revisions and add new one', async () => {
    const dapId = '1234';
    const dapName = 'DashPay';

    const firstReference = new Reference();
    const firstSchema = {};
    const firstVersion = 1;
    const firstPreviousVersions = [];
    const firstDapContractVersion = new DapContract(
      dapId,
      dapName,
      firstReference,
      firstSchema,
      firstVersion,
      firstPreviousVersions,
    );

    const secondReference = new Reference();
    const secondSchema = {};
    const secondVersion = 2;
    const secondPreviousVersions = [firstDapContractVersion.currentRevision()];
    const secondDapContractVersion = new DapContract(
      dapId,
      dapName,
      secondReference,
      secondSchema,
      secondVersion,
      secondPreviousVersions,
    );
    await dapContractRepository.store(secondDapContractVersion);

    const packet = getTransitionPacketFixtures()[0];
    const thirdVersion = 3;
    const thirdDapContractData = packet.dapcontract;
    thirdDapContractData.dapver = thirdVersion;
    thirdDapContractData.upgradedapid = dapId;
    const thirdReference = new Reference();

    await updateDapContract(dapId, thirdReference, thirdDapContractData);
    const thirdDapContractEntity = await dapContractRepository.find(dapId);

    expect(thirdDapContractEntity.getSchema()).to.deep.equal(thirdDapContractData.dapschema);
    expect(thirdDapContractEntity.getVersion()).to.deep.equal(thirdDapContractData.dapver);
    expect(thirdDapContractEntity.getPreviousVersions()).to.deep.equal([
      firstDapContractVersion.currentRevision(),
      secondDapContractVersion.currentRevision(),
    ]);
  });

  it('should not store DapContract if DapContract with upgrade dap id is not found', async () => {
    const dapId = '1234';

    const packet = getTransitionPacketFixtures()[0];
    const thirdVersion = 3;
    const dapContractData = packet.dapcontract;
    dapContractData.dapver = thirdVersion;
    dapContractData.upgradedapid = dapId;
    const reference = new Reference();

    await updateDapContract(dapId, reference, dapContractData);
    const thirdDapContractEntity = await dapContractRepository.find(dapId);

    expect(thirdDapContractEntity).to.be.null();
  });
});
