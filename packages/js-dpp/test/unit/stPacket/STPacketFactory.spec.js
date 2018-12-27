const rewiremock = require('rewiremock/node');

const AbstractDataProvider = require('../../../lib/dataProvider/AbstractDataProvider');

const STPacket = require('../../../lib/stPacket/STPacket');

const getDapContractFixture = require('../../../lib/test/fixtures/getDapContractFixture');
const getSTPacketFixture = require('../../../lib/test/fixtures/getSTPacketFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidSTPacketError = require('../../../lib/stPacket/errors/InvalidSTPacketError');
const InvalidSTPacketContractIdError = require('../../../lib/errors/InvalidSTPacketContractIdError');
const ConsensusError = require('../../../lib/errors/ConsensusError');

describe('STPacketFactory', () => {
  let decodeMock;
  let STPacketFactory;
  let validateSTPacketMock;
  let createDapContractMock;
  let fetchDapContractMock;
  let dataProviderMock;
  let dapContract;
  let factory;
  let dapContractId;
  let stPacket;
  let rawSTPacket;

  beforeEach(function beforeEach() {
    decodeMock = this.sinonSandbox.stub();
    validateSTPacketMock = this.sinonSandbox.stub();
    createDapContractMock = this.sinonSandbox.stub();

    dataProviderMock = this.sinonSandbox.createStubInstance(AbstractDataProvider, {
      fetchDapContract: this.sinonSandbox.stub(),
    });
    fetchDapContractMock = dataProviderMock.fetchDapContract;

    // Require STPacketFactory for webpack
    // eslint-disable-next-line global-require
    require('../../../lib/stPacket/STPacketFactory');

    STPacketFactory = rewiremock.proxy('../../../lib/stPacket/STPacketFactory', {
      '../../../lib/util/serializer': { decode: decodeMock },
      '../../../lib/stPacket/STPacket': STPacket,
    });

    dapContract = getDapContractFixture();

    dapContractId = dapContract.getId();

    factory = new STPacketFactory(
      dataProviderMock,
      validateSTPacketMock,
      createDapContractMock,
    );

    stPacket = getSTPacketFixture();

    rawSTPacket = stPacket.toJSON();
  });

  describe('create', () => {
    it('should return new STPacket', () => {
      const newSTPacket = factory.create(dapContractId, dapContract);

      expect(newSTPacket).to.be.instanceOf(STPacket);

      expect(newSTPacket.getDapContractId()).to.be.equal(dapContractId);
    });
  });

  describe('createFromObject', () => {
    it('should return new STPacket with DAP Objects', async () => {
      validateSTPacketMock.returns(new ValidationResult());
      fetchDapContractMock.resolves(dapContract);

      rawSTPacket = stPacket.toJSON();

      const result = await factory.createFromObject(rawSTPacket);

      expect(result).to.be.instanceOf(STPacket);

      expect(result.toJSON()).to.be.deep.equal(rawSTPacket);

      expect(fetchDapContractMock).to.be.calledOnceWith(rawSTPacket.contractId);

      expect(validateSTPacketMock).to.be.calledOnceWith(rawSTPacket, dapContract);
    });

    it('should throw error if STPacket has invalid contract ID', async () => {
      let error;
      try {
        await factory.createFromObject(rawSTPacket);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(InvalidSTPacketError);

      expect(error.getErrors()).to.have.length(1);

      const [consensusError] = error.getErrors();
      expect(consensusError).to.be.instanceOf(InvalidSTPacketContractIdError);
      expect(consensusError.getDapContractId()).to.be.equal(rawSTPacket.contractId);
      expect(consensusError.getDapContract()).to.be.undefined();

      expect(fetchDapContractMock).to.be.calledOnceWith(rawSTPacket.contractId);
      expect(validateSTPacketMock).not.to.be.called();
    });

    it('should return new STPacket with DAP Contract', async () => {
      validateSTPacketMock.returns(new ValidationResult());

      createDapContractMock.returns(dapContract);

      stPacket.setDapObjects([]);
      stPacket.setDapContract(dapContract);

      rawSTPacket = stPacket.toJSON();

      const result = await factory.createFromObject(rawSTPacket);

      expect(result).to.be.instanceOf(STPacket);

      expect(result.toJSON()).to.be.deep.equal(rawSTPacket);

      expect(fetchDapContractMock).not.to.be.called();

      expect(validateSTPacketMock).to.be.calledOnceWith(rawSTPacket);
    });

    it('should throw error if passed object is not valid', async () => {
      fetchDapContractMock.returns(dapContract);

      const validationError = new ConsensusError('test');

      validateSTPacketMock.returns(new ValidationResult([validationError]));

      let error;
      try {
        await factory.createFromObject(rawSTPacket);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(InvalidSTPacketError);
      expect(error.getErrors()).to.have.length(1);

      const [consensusError] = error.getErrors();

      expect(consensusError).to.be.equal(validationError);

      expect(validateSTPacketMock).to.be.calledOnceWith(rawSTPacket);
    });
  });

  describe('createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new DapContract from serialized DapContract', async () => {
      const serializedSTPacket = stPacket.serialize();

      decodeMock.returns(rawSTPacket);

      factory.createFromObject.resolves(stPacket);

      const result = await factory.createFromSerialized(serializedSTPacket);

      expect(result).to.be.equal(stPacket);

      expect(factory.createFromObject).to.be.calledOnceWith(rawSTPacket);

      expect(decodeMock).to.be.calledOnceWith(serializedSTPacket);
    });
  });

  describe('setDataProvider', () => {
    it('should set DataProvider', () => {
      factory.dataProvider = null;

      const result = factory.setDataProvider(dataProviderMock);

      expect(result).to.be.equal(factory);
      expect(factory.dataProvider).to.be.equal(dataProviderMock);
    });
  });

  describe('getDataProvider', () => {
    it('should return DataProvider', () => {
      const result = factory.getDataProvider();

      expect(result).to.be.equal(dataProviderMock);
    });
  });
});
