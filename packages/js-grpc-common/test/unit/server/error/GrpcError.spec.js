const GrpcError = require('../../../../lib/server/error/GrpcError');

describe('GrpcError', () => {
  let code;
  let message;
  let metadata;
  let error;

  beforeEach(() => {
    code = 1;
    message = 'Message';
    metadata = {};

    error = new GrpcError(code, message, metadata);
  });

  describe('#getMessage', () => {
    it('should return message', () => {
      const result = error.getMessage();

      expect(result).to.equal(message);
    });
  });

  describe('#getCode', () => {
    it('should return code', () => {
      const result = error.getCode();

      expect(result).to.equal(code);
    });
  });

  describe('#getRawMetadata', () => {
    it('should return metadata', () => {
      const result = error.getRawMetadata();

      expect(result).to.equal(metadata);
    });
  });

  describe('#setMessage', () => {
    it('should set message', async () => {
      message = 'error message';
      error.setMessage(message);

      expect(error.getMessage()).to.equal(message);
    });
  });

  describe('#setRawMetadata', () => {
    it('should set metadata', async () => {
      metadata = {
        stack: 'stack info',
      };

      error.setRawMetadata(metadata);

      expect(error.getRawMetadata()).to.deep.equal(metadata);
    });
  });
});
