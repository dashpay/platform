const { Transaction } = require('@dashevo/dashcore-lib');

const DashApplicationProtocol = require('../../../lib/DashApplicationProtocol');

const STPacket = require('../../../lib/stPacket/STPacket');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getSTPacketFixture = require('../../../lib/test/fixtures/getSTPacketFixture');
const getDapContractFixture = require('../../../lib/test/fixtures/getDapContractFixture');

const createDataProviderMock = require('../../../lib/test/mocks/createDataProviderMock');

const MissingOptionError = require('../../../lib/errors/MissingOptionError');

describe('STPacketFacade', () => {
  let dap;
  let stPacket;
  let dapContract;
  let dataProviderMock;

  beforeEach(function beforeEach() {
    dapContract = getDapContractFixture();

    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    dataProviderMock.fetchDapContract.resolves(dapContract);
    dataProviderMock.fetchTransaction.resolves(null);
    dataProviderMock.fetchDapObjects.resolves([]);

    stPacket = getSTPacketFixture();

    dap = new DashApplicationProtocol({
      userId: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
      dapContract,
      dataProvider: dataProviderMock,
    });
  });

  describe('create', () => {
    it('should create ST Packet', () => {
      const result = dap.packet.create(stPacket.getDapObjects());

      expect(result).to.be.instanceOf(STPacket);

      expect(result.getDapContractId()).to.be.equal(stPacket.getDapContractId());
      expect(result.getDapObjects()).to.be.deep.equal(stPacket.getDapObjects());
    });

    it('should throw error if DAP Contract is not defined', () => {
      dap = new DashApplicationProtocol();

      let error;
      try {
        dap.packet.create(stPacket.objects);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('dapContract');
    });
  });

  describe('createFromObject', () => {
    it('should create ST Packet from plain object', async () => {
      const result = await dap.packet.createFromObject(stPacket.toJSON());

      expect(result).to.be.instanceOf(STPacket);

      expect(result.toJSON()).to.be.deep.equal(stPacket.toJSON());
    });

    it('should throw error if DataProvider is not defined', async () => {
      dap = new DashApplicationProtocol();

      let error;
      try {
        await dap.packet.createFromObject(stPacket.toJSON());
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('dataProvider');
    });
  });

  describe('createFromSerialized', () => {
    it('should create ST Packet from string', async () => {
      const result = await dap.packet.createFromSerialized(stPacket.serialize());

      expect(result).to.be.instanceOf(STPacket);

      expect(result.toJSON()).to.be.deep.equal(stPacket.toJSON());
    });

    it('should throw error if DataProvider is not defined', async () => {
      dap = new DashApplicationProtocol();

      let error;
      try {
        await dap.packet.createFromSerialized(stPacket.serialize());
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('dataProvider');
    });
  });

  describe('validate', () => {
    it('should validate ST Packet', () => {
      const result = dap.packet.validate(stPacket);

      expect(result).to.be.instanceOf(ValidationResult);
    });

    it('should throw error if Dap Contract is not defined', () => {
      dap = new DashApplicationProtocol();

      let error;
      try {
        dap.packet.validate(stPacket);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('dapContract');
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
      const result = await dap.packet.verify(stPacket, stateTransition);

      expect(result).to.be.instanceOf(ValidationResult);
    });

    it('should throw error if DataProvider is not defined', async () => {
      dap = new DashApplicationProtocol();

      let error;
      try {
        await dap.packet.verify(stPacket, stateTransition);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('dataProvider');
    });
  });
});
