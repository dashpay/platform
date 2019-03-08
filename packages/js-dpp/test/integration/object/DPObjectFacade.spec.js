const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const DPObject = require('../../../lib/object/DPObject');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getDPObjectsFixture = require('../../../lib/test/fixtures/getDPObjectsFixture');
const getDPContractFixture = require('../../../lib/test/fixtures/getDPContractFixture');

const MissingOptionError = require('../../../lib/errors/MissingOptionError');

describe('DPObjectFacade', () => {
  let dpp;
  let dpObject;
  let dpContract;

  beforeEach(() => {
    dpContract = getDPContractFixture();

    dpp = new DashPlatformProtocol({
      userId: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
      dpContract,
    });

    ([dpObject] = getDPObjectsFixture());
  });

  describe('create', () => {
    it('should create DP Object', () => {
      const result = dpp.object.create(
        dpObject.getType(),
        dpObject.getData(),
      );

      expect(result).to.be.an.instanceOf(DPObject);

      expect(result.getType()).to.equal(dpObject.getType());
      expect(result.getData()).to.deep.equal(dpObject.getData());
    });

    it('should throw an error if User ID is not defined', () => {
      dpp = new DashPlatformProtocol({
        dpContract,
      });

      let error;
      try {
        dpp.object.create(
          dpObject.getType(),
          dpObject.getData(),
        );
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.equal('userId');
    });

    it('should throw an error if DP Contract is not defined', () => {
      dpp = new DashPlatformProtocol({
        userId: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
      });

      let error;
      try {
        dpp.object.create(
          dpObject.getType(),
          dpObject.getData(),
        );
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.equal('dpContract');
    });
  });

  describe('createFromObject', () => {
    it('should create DP Object from plain object', () => {
      const result = dpp.object.createFromObject(dpObject.toJSON());

      expect(result).to.be.an.instanceOf(DPObject);

      expect(result.toJSON()).to.deep.equal(dpObject.toJSON());
    });

    it('should throw an error if User ID is not defined', () => {
      dpp = new DashPlatformProtocol({
        dpContract,
      });

      let error;
      try {
        dpp.object.createFromObject(dpObject.toJSON());
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.equal('userId');
    });

    it('should throw an error if DP Contract is not defined', () => {
      dpp = new DashPlatformProtocol({
        userId: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
      });

      let error;
      try {
        dpp.object.createFromObject(dpObject.toJSON());
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.equal('dpContract');
    });
  });

  describe('createFromSerialized', () => {
    it('should create DP Object from string', () => {
      const result = dpp.object.createFromSerialized(dpObject.serialize());

      expect(result).to.be.an.instanceOf(DPObject);

      expect(result.toJSON()).to.deep.equal(dpObject.toJSON());
    });

    it('should throw an error if User ID is not defined', () => {
      dpp = new DashPlatformProtocol({
        dpContract,
      });

      let error;
      try {
        dpp.object.createFromSerialized(dpObject.serialize());
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.equal('userId');
    });

    it('should throw an error if DP Contract is not defined', () => {
      dpp = new DashPlatformProtocol({
        userId: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
      });

      let error;
      try {
        dpp.object.createFromSerialized(dpObject.serialize());
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.equal('dpContract');
    });
  });

  describe('validate', () => {
    it('should validate DP Object', () => {
      const result = dpp.object.validate(dpObject.toJSON());

      expect(result).to.be.an.instanceOf(ValidationResult);
    });
  });
});
