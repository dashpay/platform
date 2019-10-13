const rewiremock = require('rewiremock/node');

const STPacket = require('../../../lib/stPacket/STPacket');

const getContractFixture = require('../../../lib/test/fixtures/getContractFixture');
const getSTPacketFixture = require('../../../lib/test/fixtures/getSTPacketFixture');

const createDataProviderMock = require('../../../lib/test/mocks/createDataProviderMock');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidSTPacketError = require('../../../lib/stPacket/errors/InvalidSTPacketError');
const ContractNotPresentError = require('../../../lib/errors/ContractNotPresentError');
const ConsensusError = require('../../../lib/errors/ConsensusError');

describe.skip('STPacketFactory', () => {
  let decodeMock;
  let STPacketFactory;
  let validateSTPacketMock;
  let createContractMock;
  let dataProviderMock;
  let contract;
  let factory;
  let contractId;
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
    createContractMock = this.sinonSandbox.stub();
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

    contract = getContractFixture();

    contractId = contract.getId();

    factory = new STPacketFactory(
      dataProviderMock,
      validateSTPacketMock,
      createContractMock,
    );

    stPacket = getSTPacketFixture();

    rawSTPacket = stPacket.toJSON();
  });

  describe('create', () => {
    it('should return new STPacket', () => {
      const newSTPacket = factory.create(contractId, contract);

      expect(newSTPacket).to.be.an.instanceOf(STPacket);

      expect(newSTPacket.getContractId()).to.equal(contractId);
    });
  });

  describe('createFromObject', () => {
    it('should return new STPacket with Documents', async () => {
      // TODO: Mocks aren't working properly for this test
      // This functionality is also tested in integration/stPacket/STPacketFacade
      // hashMock.returns('14207b92f112bc674f32a8d04008d5c62f18d5b6c846acb0edfaf9f0b32fc293');
      // getMerkleRootMock.returns(
      //    Buffer.from('44207b92f112bc674f32a8d04008d5c62f18d5b6c846acb0edfaf9f0b32fc292', 'hex')
      // );
      // getMerkleTreeMock.returns([
      //    Buffer.from('14207b92f112bc674f32a8d04008d5c62f18d5b6c846acb0edfaf9f0b32fc293', 'hex')
      // ]);

      validateSTPacketMock.returns(new ValidationResult());

      dataProviderMock.fetchContract.resolves(contract);

      const result = await factory.createFromObject(rawSTPacket);

      expect(result).to.be.an.instanceOf(STPacket);

      // Solving problem described above
      const createdRawSTPacket = result.toJSON();
      createdRawSTPacket.itemsHash = rawSTPacket.itemsHash;
      createdRawSTPacket.itemsMerkleRoot = rawSTPacket.itemsMerkleRoot;

      expect(createdRawSTPacket).to.deep.equal(rawSTPacket);

      expect(dataProviderMock.fetchContract).to.have.been.calledOnceWith(rawSTPacket.contractId);

      expect(validateSTPacketMock).to.have.been.calledOnceWith(rawSTPacket, contract);
    });

    it('should return new STPacket without validation if "skipValidation" option is passed', async () => {
      // TODO: Mocks aren't working properly for this test
      // This functionality is also tested in integration/stPacket/STPacketFacade
      // hashMock.returns('14207b92f112bc674f32a8d04008d5c62f18d5b6c846acb0edfaf9f0b32fc293');
      // getMerkleRootMock.returns(
      //    Buffer.from('44207b92f112bc674f32a8d04008d5c62f18d5b6c846acb0edfaf9f0b32fc292', 'hex')
      // );
      // getMerkleTreeMock.returns([
      //    Buffer.from('14207b92f112bc674f32a8d04008d5c62f18d5b6c846acb0edfaf9f0b32fc293', 'hex')
      // ]);

      const result = await factory.createFromObject(rawSTPacket, { skipValidation: true });

      expect(result).to.be.an.instanceOf(STPacket);

      // Solving problem described above
      const createdRawSTPacket = result.toJSON();
      createdRawSTPacket.itemsHash = rawSTPacket.itemsHash;
      createdRawSTPacket.itemsMerkleRoot = rawSTPacket.itemsMerkleRoot;

      expect(createdRawSTPacket).to.deep.equal(rawSTPacket);

      expect(dataProviderMock.fetchContract).to.have.not.been.called();

      expect(validateSTPacketMock).to.have.not.been.called();
    });

    it('should throw an error if Contract is not present with contract ID specified in ST Packet', async () => {
      let error;
      try {
        await factory.createFromObject(rawSTPacket);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidSTPacketError);

      expect(error.getErrors()).to.have.length(1);
      expect(error.getRawSTPacket()).to.equal(rawSTPacket);

      const [consensusError] = error.getErrors();
      expect(consensusError).to.be.an.instanceOf(ContractNotPresentError);
      expect(consensusError.getContractId()).to.equal(rawSTPacket.contractId);

      expect(dataProviderMock.fetchContract).to.have.been.calledOnceWith(rawSTPacket.contractId);
      expect(validateSTPacketMock).to.have.not.been.called();
    });

    it('should return new STPacket with Contract', async () => {
      validateSTPacketMock.returns(new ValidationResult());

      createContractMock.returns(contract);

      stPacket.setDocuments([]);
      stPacket.setContract(contract);

      rawSTPacket = stPacket.toJSON();

      const result = await factory.createFromObject(rawSTPacket);

      expect(result).to.be.an.instanceOf(STPacket);

      expect(result.toJSON()).to.deep.equal(rawSTPacket);

      expect(dataProviderMock.fetchContract).to.have.not.been.called();

      expect(validateSTPacketMock).to.have.been.calledOnceWith(rawSTPacket);
    });

    it('should throw an error if passed object is not valid', async () => {
      dataProviderMock.fetchContract.resolves(contract);

      const validationError = new ConsensusError('test');

      validateSTPacketMock.returns(new ValidationResult([validationError]));

      let error;
      try {
        await factory.createFromObject(rawSTPacket);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidSTPacketError);

      expect(error.getErrors()).to.have.length(1);
      expect(error.getRawSTPacket()).to.equal(rawSTPacket);

      const [consensusError] = error.getErrors();

      expect(consensusError).to.equal(validationError);

      expect(validateSTPacketMock).to.have.been.calledOnceWith(rawSTPacket);
    });
  });

  describe('createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new Contract from serialized Contract', async () => {
      const serializedSTPacket = stPacket.serialize();

      decodeMock.returns(rawSTPacket);

      factory.createFromObject.resolves(stPacket);

      const result = await factory.createFromSerialized(serializedSTPacket);

      expect(result).to.equal(stPacket);

      expect(factory.createFromObject).to.have.been.calledOnceWith(rawSTPacket);

      expect(decodeMock).to.have.been.calledOnceWith(serializedSTPacket);
    });
  });

  describe('setDataProvider', () => {
    it('should set DataProvider', () => {
      factory.dataProvider = null;

      const result = factory.setDataProvider(dataProviderMock);

      expect(result).to.equal(factory);
      expect(factory.dataProvider).to.equal(dataProviderMock);
    });
  });

  describe('getDataProvider', () => {
    it('should return DataProvider', () => {
      const result = factory.getDataProvider();

      expect(result).to.equal(dataProviderMock);
    });
  });
});
