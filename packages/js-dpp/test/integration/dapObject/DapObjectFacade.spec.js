const DashApplicationProtocol = require('../../../lib/DashApplicationProtocol');

const DapObject = require('../../../lib/dapObject/DapObject');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getDapObjectsFixture = require('../../../lib/test/fixtures/getDapObjectsFixture');
const getDapContractFixture = require('../../../lib/test/fixtures/getDapContractFixture');

const MissingOptionError = require('../../../lib/errors/MissingOptionError');

describe('DapObjectFacade', () => {
  let dap;
  let dapObject;
  let dapContract;

  beforeEach(() => {
    dapContract = getDapContractFixture();

    dap = new DashApplicationProtocol({
      userId: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
      dapContract,
    });

    ([dapObject] = getDapObjectsFixture());
  });

  describe('create', () => {
    it('should create DAP Object', () => {
      const result = dap.object.create(
        dapObject.getType(),
        dapObject.getData(),
      );

      expect(result).to.be.instanceOf(DapObject);

      expect(result.getType()).to.be.equal(dapObject.getType());
      expect(result.getData()).to.be.deep.equal(dapObject.getData());
    });

    it('should throw error if User ID is not defined', () => {
      dap = new DashApplicationProtocol({
        dapContract,
      });

      let error;
      try {
        dap.object.create(
          dapObject.getType(),
          dapObject.getData(),
        );
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('userId');
    });

    it('should throw error if DAP Contract is not defined', () => {
      dap = new DashApplicationProtocol({
        userId: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
      });

      let error;
      try {
        dap.object.create(
          dapObject.getType(),
          dapObject.getData(),
        );
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('dapContract');
    });
  });

  describe('createFromObject', () => {
    it('should create DAP Object from plain object', () => {
      const result = dap.object.createFromObject(dapObject.toJSON());

      expect(result).to.be.instanceOf(DapObject);

      expect(result.toJSON()).to.be.deep.equal(dapObject.toJSON());
    });

    it('should throw error if User ID is not defined', () => {
      dap = new DashApplicationProtocol({
        dapContract: getDapContractFixture(),
      });

      let error;
      try {
        dap.object.createFromObject(dapObject.toJSON());
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('userId');
    });

    it('should throw error if DAP Contract is not defined', () => {
      dap = new DashApplicationProtocol({
        userId: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
      });

      let error;
      try {
        dap.object.createFromObject(dapObject.toJSON());
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('dapContract');
    });
  });

  describe('createFromSerialized', () => {
    it('should create DAP Object from string', () => {
      const result = dap.object.createFromSerialized(dapObject.serialize());

      expect(result).to.be.instanceOf(DapObject);

      expect(result.toJSON()).to.be.deep.equal(dapObject.toJSON());
    });

    it('should throw error if User ID is not defined', () => {
      dap = new DashApplicationProtocol({
        dapContract,
      });

      let error;
      try {
        dap.object.createFromSerialized(dapObject.serialize());
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('userId');
    });

    it('should throw error if DAP Contract is not defined', () => {
      dap = new DashApplicationProtocol({
        userId: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
      });

      let error;
      try {
        dap.object.createFromSerialized(dapObject.serialize());
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.be.equal('dapContract');
    });
  });

  describe('validate', () => {
    it('should validate DAP Object', () => {
      const result = dap.object.validate(dapObject.toJSON());

      expect(result).to.be.instanceOf(ValidationResult);
    });
  });
});
