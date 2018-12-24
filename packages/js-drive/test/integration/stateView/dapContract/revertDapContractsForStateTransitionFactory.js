const {
  mocha: {
    startMongoDb,
    startIPFS,
  },
} = require('@dashevo/js-evo-services-ctl');

const ReaderMediator = require('../../../../lib/blockchain/reader/BlockchainReaderMediator');

const Reference = require('../../../../lib/stateView/Reference');
const DapContract = require('../../../../lib/stateView/dapContract/DapContract');
const DapContractMongoDbRepository = require('../../../../lib/stateView/dapContract/DapContractMongoDbRepository');

const getBlockFixtures = require('../../../../lib/test/fixtures/getBlockFixtures');
const getTransitionPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');
const getTransitionHeaderFixtures = require('../../../../lib/test/fixtures/getTransitionHeaderFixtures');

const serializer = require('../../../../lib/util/serializer');

const RpcClientMock = require('../../../../lib/test/mock/RpcClientMock');
const ReaderMediatorMock = require('../../../../lib/test/mock/BlockchainReaderMediatorMock');

const StateTransitionPacketIpfsRepository = require('../../../../lib/storage/stPacket/StateTransitionPacketIpfsRepository');
const addSTPacketFactory = require('../../../../lib/storage/stPacket/addSTPacketFactory');
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
  let readerMediator;
  let revertDapContractsForStateTransition;
  beforeEach(function beforeEach() {
    const stPacketRepository = new StateTransitionPacketIpfsRepository(
      ipfsClient,
      1000,
    );
    addSTPacket = addSTPacketFactory(stPacketRepository);
    dapContractMongoDbRepository = new DapContractMongoDbRepository(mongoDb, serializer);
    const updateDapContract = updateDapContractFactory(dapContractMongoDbRepository);
    readerMediator = new ReaderMediatorMock(this.sinon);
    applyStateTransition = applyStateTransitionFactory(
      stPacketRepository,
      updateDapContract,
      null,
      readerMediator,
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
      readerMediator,
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
        null,
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
      {
        dapname: dapName,
        dapver: dapContractVersions.length,
      },
      dapContractVersions[dapContractVersions.length - 1].reference,
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

    expect(readerMediator.emitSerial.getCall(1)).to.be.calledWith(
      ReaderMediator.EVENTS.DAP_CONTRACT_REVERTED,
      {
        userId: lastDapContractVersion.header.extraPayload.regTxId,
        dapId,
        reference: lastDapContractVersion.reference,
        contract: dapContract.getOriginalData(),
        previousVersion: previousVersions[previousVersions.length - 1],
      },
    );
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

    const version = 1;
    const data = {
      dapname: dapName,
      dapver: version,
    };
    const deleted = false;
    const previousVersions = [];
    const dapContract = new DapContract(
      dapId,
      data,
      reference,
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

    expect(readerMediator.emitSerial).to.be.calledWith(
      ReaderMediator.EVENTS.DAP_CONTRACT_MARKED_DELETED,
      {
        userId: header.extraPayload.regTxId,
        dapId,
        reference,
        contract: data,
      },
    );
  });
});
