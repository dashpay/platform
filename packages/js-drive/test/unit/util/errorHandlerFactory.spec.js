const errorHandlerFactory = require('../../../lib/errorHandlerFactory');
const LoggerMock = require('../../../lib/test/mock/LoggerMock');

describe('errorHandlerFactory', () => {
  let errorHandler;
  let containerMock;
  let loggerMock;

  beforeEach(function beforeEach() {
    this.sinon.stub(console, 'log');
    this.sinon.stub(process, 'exit');

    containerMock = {
      dispose: this.sinon.stub(),
    };

    loggerMock = new LoggerMock(this.sinon);

    errorHandler = errorHandlerFactory(
      loggerMock,
      containerMock,
    );
  });

  it('should log error, dispose container and exit process', async () => {
    const error = new Error('message');

    await errorHandler(error);

    expect(loggerMock.fatal).to.be.calledOnceWithExactly(error);

    expect(containerMock.dispose).to.be.calledOnceWithExactly();

    expect(process.exit).to.be.calledOnceWithExactly(1);

    // eslint-disable-next-line no-console
    expect(console.log).to.be.calledOnce();
  });

  it('should use consensus logger if it\'s present', async function it() {
    const error = new Error('message');

    error.consensusLogger = new LoggerMock(this.sinon);

    await errorHandler(error);

    expect(loggerMock.fatal).to.not.be.called();
    expect(error.consensusLogger.fatal).to.be.calledOnceWithExactly(error);

    expect(containerMock.dispose).to.be.calledOnceWithExactly();

    expect(process.exit).to.be.calledOnceWithExactly(1);

    // eslint-disable-next-line no-console
    expect(console.log).to.be.calledOnce();
  });
});
