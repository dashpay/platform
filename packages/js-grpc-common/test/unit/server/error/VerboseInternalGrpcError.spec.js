const InternalGrpcError = require('../../../../lib/server/error/InternalGrpcError');
const VerboseInternalGrpcError = require('../../../../lib/server/error/VerboseInternalGrpcError');

describe('InvalidArgumentGrpcError', () => {
  let message;
  let metadata;
  let error;
  let internalError;

  beforeEach(() => {
    message = 'VerboseInternalGrpcError Test Message';
    metadata = {};

    error = new Error(message);
    internalError = new InternalGrpcError(error, metadata);
  });

  describe('constructor', () => {
    it('should attach full stack if errorPath can not be extracted from original stack', () => {
      error.stack = 'anonymous';
      internalError = new InternalGrpcError(error, metadata);
      const err = new VerboseInternalGrpcError(internalError);

      expect(err.getMessage()).to.be.equal(`${message} ${error.stack}`);
    });

    it('should attach last line of stack if it can be extracted from original stack', () => {
      const err = new VerboseInternalGrpcError(internalError);
      const [, errorPath] = error.stack.toString().split(/\r\n|\n/);

      expect(err.getMessage()).to.be.equal(`${message} ${errorPath.trim()}`);
    });
  });
});
