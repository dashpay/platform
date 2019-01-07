const { Transaction } = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const STPacket = require('../../../lib/stPacket/STPacket');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getSTPacketFixture = require('../../../lib/test/fixtures/getSTPacketFixture');
const getDPContractFixture = require('../../../lib/test/fixtures/getDPContractFixture');

const createDataProviderMock = require('../../../lib/test/mocks/createDataProviderMock');

const MissingOptionError = require('../../../lib/errors/MissingOptionError');

describe('STPacketFacade', () => {
  let dpp;
  let stPacket;
  let dpContract;
  let dataProviderMock;

  beforeEach(function beforeEach() {
    dpContract = getDPContractFixture();

    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    dataProviderMock.fetchDPContract.resolves(dpContract);
    dataProviderMock.fetchTransaction.resolves(null);
    dataProviderMock.fetchDPObjects.resolves([]);

    stPacket = getSTPacketFixture();

    dpp = new DashPlatformProtocol({
      userId: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
      dpContract,
      dataProvider: dataProviderMock,
    });
  });

  describe('create', () => {
    it('should create ST Packet', () => {
      const result = dpp.packet.create(stPacket.getDPObjects());

      expect(result).to.be.instanceOf(STPacket);

      expect(result.getDPContractId()).to.be.equal(stPacket.getDPContractId());
      expect(result.getDPObjects()).to.be.deep.equal(stPacket.getDPObjects());
    });

    it('should throw error if DP Contract is not defined', () => {
      dpp = new DashPlatformProtocol();

      let error;
      try {
        dpp.packet.create(stPacket.objects);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('dpContract');
    });
  });

  describe('createFromObject', () => {
    it('should create ST Packet from plain object', async () => {
      const result = await dpp.packet.createFromObject(stPacket.toJSON());

      expect(result).to.be.instanceOf(STPacket);

      expect(result.toJSON()).to.be.deep.equal(stPacket.toJSON());
    });

    it('should throw error if DataProvider is not defined', async () => {
      dpp = new DashPlatformProtocol();

      let error;
      try {
        await dpp.packet.createFromObject(stPacket.toJSON());
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('dataProvider');
    });
  });

  describe('createFromSerialized', () => {
    it('should create ST Packet from string', async () => {
      const result = await dpp.packet.createFromSerialized(stPacket.serialize());

      expect(result).to.be.instanceOf(STPacket);

      expect(result.toJSON()).to.be.deep.equal(stPacket.toJSON());
    });

    it('should throw error if DataProvider is not defined', async () => {
      dpp = new DashPlatformProtocol();

      let error;
      try {
        await dpp.packet.createFromSerialized(stPacket.serialize());
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('dataProvider');
    });
  });

  describe('validate', () => {
    it('should validate ST Packet', () => {
      const result = dpp.packet.validate(stPacket);

      expect(result).to.be.instanceOf(ValidationResult);
    });

    it('should throw error if DP Contract is not defined', () => {
      dpp = new DashPlatformProtocol();

      let error;
      try {
        dpp.packet.validate(stPacket);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('dpContract');
    });
  });

  describe('verify', () => {
    let stateTransition;

    beforeEach(() => {
      stateTransition = new Transaction({
        type: Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION,
        extraPayload: {
          version: 1,
          hashSTPacket: stPacket.hash(),
          regTxId: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
          creditFee: 1001,
          hashPrevSubTx: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
        },
      });
    });

    it('should verify ST Packet', async () => {
      const result = await dpp.packet.verify(stPacket, stateTransition);

      expect(result).to.be.instanceOf(ValidationResult);
    });

    it('should throw error if DataProvider is not defined', async () => {
      dpp = new DashPlatformProtocol();

      let error;
      try {
        await dpp.packet.verify(stPacket, stateTransition);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('dataProvider');
    });
  });
});
