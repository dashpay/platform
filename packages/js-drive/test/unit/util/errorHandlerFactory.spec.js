const errorHandlerFactory = require('../../../lib/errorHandlerFactory');
const LoggerMock = require('../../../lib/test/mock/LoggerMock');

describe('errorHandlerFactory', () => {
  let errorHandler;
  let containerMock;
  let loggerMock;
  let closeAbciServerMock;

  beforeEach(function beforeEach() {
    this.sinon.stub(console, 'log');
    this.sinon.stub(console, 'error');
    this.sinon.stub(process, 'exit');

    containerMock = {
      dispose: this.sinon.stub(),
    };

    closeAbciServerMock = this.sinon.stub();

    loggerMock = new LoggerMock(this.sinon);

    errorHandler = errorHandlerFactory(
      loggerMock,
      containerMock,
      closeAbciServerMock,
    );
  });

  it('should close server, log error, dispose container and exit process on first call', async () => {
    const error = new Error('message');

    await errorHandler(error);

    expect(closeAbciServerMock).to.be.calledOnceWithExactly();

    // Error face is printed
    // eslint-disable-next-line no-console
    expect(console.log).to.be.calledOnce();

    expect(loggerMock.fatal).to.be.calledOnceWithExactly({ err: error }, error.message);

    expect(containerMock.dispose).to.be.calledOnceWithExactly();

    expect(process.exit).to.be.calledOnceWithExactly(1);
  });

  it('should use consensus logger if it\'s present', async function it() {
    const error = new Error('message');

    error.consensusLogger = new LoggerMock(this.sinon);

    await errorHandler(error);

    expect(loggerMock.fatal).to.not.be.called();
    expect(error.consensusLogger.fatal).to.be.calledOnceWithExactly({ err: error }, error.message);

    expect(containerMock.dispose).to.be.calledOnceWithExactly();

    expect(process.exit).to.be.calledOnceWithExactly(1);

    // eslint-disable-next-line no-console
    expect(console.log).to.be.calledOnce();
  });

  it('should collect an error on second call', async () => {
    const error1 = new Error('error1');
    const error2 = new Error('error2');

    await Promise.all([
      errorHandler(error1),
      errorHandler(error2),
    ]);

    expect(closeAbciServerMock).to.be.calledOnceWithExactly();

    // Error face is printed
    // eslint-disable-next-line no-console
    expect(console.log).to.be.calledOnce();

    expect(loggerMock.fatal).to.be.calledTwice();

    expect(loggerMock.fatal.getCall(0)).to.be.calledWithExactly({ err: error1 }, error1.message);
    expect(loggerMock.fatal.getCall(1)).to.be.calledWithExactly({ err: error2 }, error2.message);

    expect(containerMock.dispose).to.be.calledOnceWithExactly();

    expect(process.exit).to.be.calledOnceWithExactly(1);
  });

  it('should dispose container and output error in console if it was thrown during error handling', async () => {
    const closeError = new Error('close server error');

    closeAbciServerMock.throws(closeError);

    const error = new Error('message');

    await errorHandler(error);

    expect(closeAbciServerMock).to.be.calledOnceWithExactly();

    // Error face is printed
    // eslint-disable-next-line no-console
    expect(console.log).to.not.be.called();

    expect(loggerMock.fatal).to.not.be.called();

    expect(containerMock.dispose).to.be.calledOnceWithExactly();

    // eslint-disable-next-line no-console
    expect(console.error).to.be.calledOnceWithExactly(closeError);

    expect(process.exit).to.be.calledOnceWithExactly(1);
  });

  it('should output error in console if it was thrown during dispose', async () => {
    const disposeError = new Error('dispose error');

    containerMock.dispose.throws(disposeError);

    const error = new Error('message');

    await errorHandler(error);

    expect(closeAbciServerMock).to.be.calledOnceWithExactly();

    // Error face is printed
    // eslint-disable-next-line no-console
    expect(console.log).to.be.calledOnce();

    expect(loggerMock.fatal).to.be.calledOnceWithExactly({ err: error }, error.message);

    expect(containerMock.dispose).to.be.calledOnceWithExactly();

    // eslint-disable-next-line no-console
    expect(console.error).to.be.calledOnceWithExactly(disposeError);

    expect(process.exit).to.be.calledOnceWithExactly(1);
  });
});
