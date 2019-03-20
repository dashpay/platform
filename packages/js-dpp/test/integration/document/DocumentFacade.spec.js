const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const Document = require('../../../lib/document/Document');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDPContractFixture = require('../../../lib/test/fixtures/getDPContractFixture');

const MissingOptionError = require('../../../lib/errors/MissingOptionError');

describe('DocumentFacade', () => {
  let dpp;
  let document;
  let dpContract;

  beforeEach(() => {
    dpContract = getDPContractFixture();

    dpp = new DashPlatformProtocol({
      userId: '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288',
      dpContract,
    });

    ([document] = getDocumentsFixture());
  });

  describe('create', () => {
    it('should create Document', () => {
      const result = dpp.document.create(
        document.getType(),
        document.getData(),
      );

      expect(result).to.be.an.instanceOf(Document);

      expect(result.getType()).to.equal(document.getType());
      expect(result.getData()).to.deep.equal(document.getData());
    });

    it('should throw an error if User ID is not defined', () => {
      dpp = new DashPlatformProtocol({
        dpContract,
      });

      let error;
      try {
        dpp.document.create(
          document.getType(),
          document.getData(),
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
        dpp.document.create(
          document.getType(),
          document.getData(),
        );
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.equal('dpContract');
    });
  });

  describe('createFromObject', () => {
    it('should create Document from plain object', () => {
      const result = dpp.document.createFromObject(document.toJSON());

      expect(result).to.be.an.instanceOf(Document);

      expect(result.toJSON()).to.deep.equal(document.toJSON());
    });

    it('should throw an error if User ID is not defined', () => {
      dpp = new DashPlatformProtocol({
        dpContract,
      });

      let error;
      try {
        dpp.document.createFromObject(document.toJSON());
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
        dpp.document.createFromObject(document.toJSON());
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.equal('dpContract');
    });
  });

  describe('createFromSerialized', () => {
    it('should create Document from string', () => {
      const result = dpp.document.createFromSerialized(document.serialize());

      expect(result).to.be.an.instanceOf(Document);

      expect(result.toJSON()).to.deep.equal(document.toJSON());
    });

    it('should throw an error if User ID is not defined', () => {
      dpp = new DashPlatformProtocol({
        dpContract,
      });

      let error;
      try {
        dpp.document.createFromSerialized(document.serialize());
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
        dpp.document.createFromSerialized(document.serialize());
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(MissingOptionError);
      expect(error.getOptionName()).to.equal('dpContract');
    });
  });

  describe('validate', () => {
    it('should validate Document', () => {
      const result = dpp.document.validate(document.toJSON());

      expect(result).to.be.an.instanceOf(ValidationResult);
    });
  });
});
