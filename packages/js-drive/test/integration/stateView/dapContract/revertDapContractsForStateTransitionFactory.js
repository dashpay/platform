const {
  mocha: {
    startMongoDb,
    startIPFS,
  },
} = require('@dashevo/js-evo-services-ctl');

const Reference = require('../../../../lib/stateView/Reference');
const DapContract = require('../../../../lib/stateView/dapContract/DapContract');
const DapContractMongoDbRepository = require('../../../../lib/stateView/dapContract/DapContractMongoDbRepository');

const getBlockFixtures = require('../../../../lib/test/fixtures/getBlockFixtures');
const getTransitionPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');
const getTransitionHeaderFixtures = require('../../../../lib/test/fixtures/getTransitionHeaderFixtures');

const sanitizeData = require('../../../../lib/mongoDb/sanitizeData');

const RpcClientMock = require('../../../../lib/test/mock/RpcClientMock');

const addSTPacketFactory = require('../../../../lib/storage/ipfs/addSTPacketFactory');
const updateDapContractFactory = require('../../../../lib/stateView/dapContract/updateDapContractFactory');
const revertDapContractsForStateTransitionFactory = require('../../../../lib/stateView/dapContract/revertDapContractsForStateTransitionFactory');
const applyStateTransitionFactory = require('../../../../lib/stateView/applyStateTransitionFactory');
const applyStateTransitionFromReferenceFactory = require('../../../../lib/stateView/applyStateTransitionFromReferenceFactory');

const doubleSha256 = require('../../../../lib/util/doubleSha256');

describe('revertDapContractsForStateTransitionFactory', () => {
  let mongoDb;
  startMongoDb().then(async (mongoDbInstance) => {
    mongoDb = await mongoDbInstance.getDb();
  });

  let ipfsClient;
  startIPFS().then(async (ipfsInstance) => {
    ipfsClient = await ipfsInstance.getApi();
  });

  let addSTPacket;
  let dapContractMongoDbRepository;
  let applyStateTransition;
  let rpcClientMock;
  let revertDapContractsForStateTransition;
  beforeEach(function beforeEach() {
    addSTPacket = addSTPacketFactory(ipfsClient);
    dapContractMongoDbRepository = new DapContractMongoDbRepository(mongoDb, sanitizeData);
    const updateDapContract = updateDapContractFactory(dapContractMongoDbRepository);
    applyStateTransition = applyStateTransitionFactory(
      ipfsClient,
      updateDapContract,
      null,
      30 * 1000,
    );
    rpcClientMock = new RpcClientMock(this.sinon);
    const applyStateTransitionFromReference = applyStateTransitionFromReferenceFactory(
      applyStateTransition,
      rpcClientMock,
    );
    revertDapContractsForStateTransition = revertDapContractsForStateTransitionFactory(
      dapContractMongoDbRepository,
      rpcClientMock,
      applyStateTransition,
      applyStateTransitionFromReference,
    );
  });

  it('should remove last version of DapContract and re-apply previous versions in order', async () => {
    const dapName = 'DashPay';

    const dapContractVersions = [];
    for (let i = 0; i < 3; i++) {
      const block = getBlockFixtures()[i];

      // User `0`-index fixture as it is DapContract
      const packet = getTransitionPacketFixtures()[0];
      if (i >= 1) {
        const versionOnePacket = dapContractVersions[0].packet;
        packet.dapcontract.upgradedapid = doubleSha256(versionOnePacket.dapcontract);
      }
      packet.dapcontract.dapver = (i + 1);
      const header = getTransitionHeaderFixtures()[i];
      header.extraPayload.hashSTPacket = packet.getHash();

      await addSTPacket(packet);

      const reference = new Reference(
        block.hash,
        block.height,
        header.hash,
        packet.getHash(),
      );

      dapContractVersions.push({
        version: (i + 1),
        block,
        header,
        packet,
        reference,
      });

      rpcClientMock.getRawTransaction
        .withArgs(header.hash)
        .resolves({
          result: header,
        });
    }

    const dapId = doubleSha256(dapContractVersions[0].packet.dapcontract);

    const previousVersions = [];
    for (let i = 0; i < dapContractVersions.length - 1; i++) {
      previousVersions.push({
        version: dapContractVersions[i].version,
        reference: dapContractVersions[i].reference,
      });
    }

    const dapContract = new DapContract(
      dapId,
      dapName,
      dapContractVersions[dapContractVersions.length - 1].reference,
      {},
      dapContractVersions.length,
      false,
      previousVersions,
    );
    await dapContractMongoDbRepository.store(dapContract);

    const lastDapContractVersion = dapContractVersions[dapContractVersions.length - 1];
    await revertDapContractsForStateTransition({
      stateTransition: lastDapContractVersion.header,
      block: lastDapContractVersion.block,
    });

    const dapContractAfter = await dapContractMongoDbRepository.find(dapId);

    expect(dapContractAfter.getVersion()).to.be.equal(2);
    expect(dapContractAfter.getPreviousVersions()).to.be.deep.equal([
      {
        version: dapContractVersions[0].version,
        reference: dapContractVersions[0].reference,
      },
    ]);
  });

  it('should delete DapContract if there are no previous versions', async () => {
    const dapId = '1234';
    const dapName = 'DashPay';

    const block = getBlockFixtures()[0];
    const header = getTransitionHeaderFixtures()[0];
    const blockHash = block.hash;
    const blockHeight = block.height;
    const stHeaderHash = header.hash;
    const stPacketHash = '';
    const objectHash = '';
    const reference = new Reference(
      blockHash,
      blockHeight,
      stHeaderHash,
      stPacketHash,
      objectHash,
    );
    const schema = {};
    const version = 1;
    const deleted = false;
    const previousVersions = [];
    const dapContract = new DapContract(
      dapId,
      dapName,
      reference,
      schema,
      version,
      deleted,
      previousVersions,
    );
    await dapContractMongoDbRepository.store(dapContract);

    await revertDapContractsForStateTransition({
      stateTransition: header,
      block,
    });

    const dapContractAfter = await dapContractMongoDbRepository.find(dapId);
    expect(dapContractAfter).to.not.exist();
  });
});
