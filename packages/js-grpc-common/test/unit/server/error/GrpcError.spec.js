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

  describe('#getMetadata', () => {
    it('should return metadata', () => {
      const result = error.getMetadata();

      expect(result).to.equal(metadata);
    });
  });
});
