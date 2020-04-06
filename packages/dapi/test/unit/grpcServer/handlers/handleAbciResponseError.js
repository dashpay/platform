const {
  server: {
    error: {
      InternalGrpcError,
      InvalidArgumentGrpcError,
      DeadlineExceededGrpcError,
      ResourceExhaustedGrpcError,
      NotFoundGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const handleAbciResponseError = require(
  '../../../../lib/grpcServer/handlers/handleAbciResponseError',
);

const AbciResponseError = require('../../../../lib/errors/AbciResponseError');

describe('handleAbciResponseError', () => {
  let message;
  let data;

  beforeEach(() => {
    message = 'message';
    data = { error: 'some data' };
  });

  [
    { code: 6, errorClass: ResourceExhaustedGrpcError },
    { code: 5, errorClass: DeadlineExceededGrpcError },
    { code: 3, errorClass: NotFoundGrpcError },
    { code: 2, errorClass: InvalidArgumentGrpcError },
    { code: 1, errorClass: InternalGrpcError },
  ].forEach(({ code, errorClass }) => {
    it(`should throw ${errorClass.name} if response code is ${code}`, () => {
      try {
        handleAbciResponseError(
          new AbciResponseError(code, { message, data }),
        );
      } catch (e) {
        expect(e).to.be.an.instanceOf(errorClass);

        if (code === 1) {
          expect(e.getMessage()).to.equal('Internal error');
        } else {
          expect(e.getMessage()).to.equal(message);
        }
      }
    });
  });
});
