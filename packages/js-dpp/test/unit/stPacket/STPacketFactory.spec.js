const rewiremock = require('rewiremock/node');

const STPacket = require('../../../lib/stPacket/STPacket');

const getDapContractFixture = require('../../../lib/test/fixtures/getDapContractFixture');
const getSTPacketFixture = require('../../../lib/test/fixtures/getSTPacketFixture');

const createDataProviderMock = require('../../../lib/test/mocks/createDataProviderMock');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidSTPacketError = require('../../../lib/stPacket/errors/InvalidSTPacketError');
const InvalidSTPacketContractIdError = require('../../../lib/errors/InvalidSTPacketContractIdError');
const ConsensusError = require('../../../lib/errors/ConsensusError');

describe('STPacketFactory', () => {
  let decodeMock;
  let STPacketFactory;
  let validateSTPacketMock;
  let createDapContractMock;
  let dataProviderMock;
  let dapContract;
  let factory;
  let dapContractId;
  let stPacket;
  let rawSTPacket;
  let encodeMock;
  let serializerMock;
  let hashMock;
  let merkleTreeUtilMock;
  let getMerkleTreeMock;
  let getMerkleRootMock;

  beforeEach(function beforeEach() {
    decodeMock = this.sinonSandbox.stub();
    encodeMock = this.sinonSandbox.stub();
    validateSTPacketMock = this.sinonSandbox.stub();
    createDapContractMock = this.sinonSandbox.stub();
    serializerMock = { encode: encodeMock, decode: decodeMock };
    hashMock = this.sinonSandbox.stub();
    getMerkleTreeMock = this.sinonSandbox.stub();
    getMerkleRootMock = this.sinonSandbox.stub();
    merkleTreeUtilMock = {
      getMerkleTree: getMerkleTreeMock,
      getMerkleRoot: getMerkleRootMock,
    };

    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    // Require STPacketFactory for webpack
    // eslint-disable-next-line global-require
    require('../../../lib/stPacket/STPacketFactory');

    STPacketFactory = rewiremock.proxy('../../../lib/stPacket/STPacketFactory', {
      '../../../lib/util/serializer': serializerMock,
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/merkleTree': merkleTreeUtilMock,
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
    it.skip('should return new STPacket with DAP Objects', async () => {
      // Mocks aren't working properly for this test
      // This functionality is also tested in integration/stPacket/STPacketFacade
      hashMock.returns('14207b92f112bc674f32a8d04008d5c62f18d5b6c846acb0edfaf9f0b32fc293');
      getMerkleRootMock.returns(Buffer.from('44207b92f112bc674f32a8d04008d5c62f18d5b6c846acb0edfaf9f0b32fc292', 'hex'));
      getMerkleTreeMock.returns([Buffer.from('14207b92f112bc674f32a8d04008d5c62f18d5b6c846acb0edfaf9f0b32fc293', 'hex')]);
      validateSTPacketMock.returns(new ValidationResult());
      dataProviderMock.fetchDapContract.resolves(dapContract);

      rawSTPacket = stPacket.toJSON();

      const result = await factory.createFromObject(rawSTPacket);

      expect(result).to.be.instanceOf(STPacket);

      expect(result.toJSON()).to.be.deep.equal(rawSTPacket);

      expect(dataProviderMock.fetchDapContract).to.be.calledOnceWith(rawSTPacket.contractId);

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
      expect(error.getRawSTPacket()).to.be.equal(rawSTPacket);

      const [consensusError] = error.getErrors();
      expect(consensusError).to.be.instanceOf(InvalidSTPacketContractIdError);
      expect(consensusError.getDapContractId()).to.be.equal(rawSTPacket.contractId);
      expect(consensusError.getDapContract()).to.be.undefined();

      expect(dataProviderMock.fetchDapContract).to.be.calledOnceWith(rawSTPacket.contractId);
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

      expect(dataProviderMock.fetchDapContract).not.to.be.called();

      expect(validateSTPacketMock).to.be.calledOnceWith(rawSTPacket);
    });

    it('should throw error if passed object is not valid', async () => {
      dataProviderMock.fetchDapContract.resolves(dapContract);

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
      expect(error.getRawSTPacket()).to.be.equal(rawSTPacket);

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
