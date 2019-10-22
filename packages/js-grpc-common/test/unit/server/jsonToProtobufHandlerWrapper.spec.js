const jsonToProtobufHandlerWrapper = require(
  '../../../lib/server/jsonToProtobufHandlerWrapper',
);

describe('jsonToProtobufHandlerWrapper', () => {
  let jsonToProtobufMock;
  let protobufToJsonMock;
  let rpcMethodMock;

  beforeEach(function beforeEach() {
    jsonToProtobufMock = this.sinon.stub();
    protobufToJsonMock = this.sinon.stub();
    rpcMethodMock = this.sinon.stub();
  });

  it('should proxy call\'s request and write', function it() {
    const message = 12;

    const call = {
      request: 41,
      write: this.sinon.stub(),
    };

    jsonToProtobufMock.returns(call.request + 1);
    protobufToJsonMock = this.sinon.spy((value) => value + 1);

    let modifiedRequest;
    rpcMethodMock = this.sinon.spy((rpcCall) => {
      modifiedRequest = rpcCall.request;
      rpcCall.write(message);
    });

    const wrappedMethod = jsonToProtobufHandlerWrapper(
      jsonToProtobufMock,
      protobufToJsonMock,
      rpcMethodMock,
    );

    wrappedMethod(call);

    expect(jsonToProtobufMock).to.have.been.calledOnceWith(call.request);
    expect(protobufToJsonMock).to.have.been.calledOnceWith(message);

    expect(call.write).to.have.been.calledOnceWith(message + 1, undefined, undefined);
    expect(modifiedRequest).to.equal(call.request + 1);
  });

  it('should proxy callback and it\'s message', function it() {
    const message = 12;

    protobufToJsonMock = this.sinon.spy((value) => value + 1);

    rpcMethodMock = this.sinon.spy((_, callback) => {
      callback(null, message);
    });

    const wrappedMethod = jsonToProtobufHandlerWrapper(
      jsonToProtobufMock,
      protobufToJsonMock,
      rpcMethodMock,
    );

    const callback = this.sinon.stub();

    wrappedMethod({}, callback);

    expect(protobufToJsonMock).to.have.been.calledOnceWith(message);
    expect(callback).to.have.been.calledOnceWith(null, message + 1);
  });
});
