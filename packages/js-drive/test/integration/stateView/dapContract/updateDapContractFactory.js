const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');
const getTransitionPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');
const doubleSha256 = require('../../../../lib/util/doubleSha256');
const Reference = require('../../../../lib/stateView/Reference');
const DapContract = require('../../../../lib/stateView/dapContract/DapContract');
const serializer = require('../../../../lib/util/serializer');
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
    dapContractRepository = new DapContractMongoDbRepository(mongoDb, serializer);
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
    expect(dapContract.getOriginalData()).to.deep.equal(dapContractData);
    expect(dapContract.getVersion()).to.deep.equal(dapContractData.dapver);
  });

  it('should maintain DapContract previous revisions and add new one', async () => {
    const dapId = '1234';

    const firstReference = new Reference(null, null, null, null, null);
    const firstData = {
      dapver: 1,
    };
    const firstDapContractVersion = new DapContract(
      dapId,
      firstData,
      firstReference,
      false,
    );

    const secondReference = new Reference(null, null, null, null, null);
    const secondData = {
      dapver: 2,
    };
    const secondDapContractVersion = new DapContract(
      dapId,
      secondData,
      secondReference,
      false,
      [firstDapContractVersion.currentRevision()],
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

    expect(thirdDapContractEntity.getOriginalData()).to.deep.equal(thirdDapContractData);
    expect(thirdDapContractEntity.getVersion()).to.deep.equal(thirdDapContractData.dapver);
    expect(thirdDapContractEntity.getPreviousVersions()).to.deep.equal([
      firstDapContractVersion.currentRevision(),
      secondDapContractVersion.currentRevision(),
    ]);
  });

  it('should remove unnecessary previous versions of DapContract upon reverting', async () => {
    const dapId = '1234';

    const firstReference = new Reference(null, null, null, null, null);
    const firstData = {
      dapver: 1,
    };
    const firstDapContractVersion = new DapContract(
      dapId,
      firstData,
      firstReference,
      false,
    );

    const secondReference = new Reference(null, null, null, null, null);
    const secondData = {
      dapver: 2,
      upgradedapid: dapId,
    };
    const secondDapContractVersion = new DapContract(
      dapId,
      secondData,
      secondReference,
      false,
      [firstDapContractVersion.currentRevision()],
    );
    await dapContractRepository.store(secondDapContractVersion);

    const packet = getTransitionPacketFixtures()[0];
    const thirdVersion = 3;
    const thirdDapContractData = packet.dapcontract;
    thirdDapContractData.dapver = thirdVersion;
    thirdDapContractData.upgradedapid = dapId;
    const thirdReference = new Reference();

    await updateDapContract(dapId, thirdReference, thirdDapContractData);

    await updateDapContract(dapId, secondReference, secondData, true);

    const secondDapContractEntity = await dapContractRepository.find(dapId);

    expect(secondDapContractEntity.getPreviousVersions()).to.deep.equal([
      firstDapContractVersion.currentRevision(),
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
