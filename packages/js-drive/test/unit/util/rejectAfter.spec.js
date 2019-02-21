const rejectAfter = require('../../../lib/util/rejectAfter');

describe('rejectAfter', () => {
  it('should return resolved promise', async () => {
    const resolvedValue = 1;
    const promise = Promise.resolve(resolvedValue);

    const actualValue = await rejectAfter(promise, new Error(), 1000);

    expect(actualValue).to.equal(resolvedValue);
  });

  it('should return rejected promise', (done) => {
    const error = new Error();
    const promise = Promise.reject(error);

    const actualPromise = rejectAfter(promise, new Error(), 1000);

    expect(actualPromise).to.be.rejectedWith(error).and.notify(done);
  });

  it('should reject unresolved promise after specified time', function it(done) {
    const promise = new Promise(() => {});
    const error = new Error();

    const clock = this.sinon.useFakeTimers();

    const rejectedPromise = rejectAfter(promise, error, 1000);

    clock.next();

    expect(rejectedPromise).to.be.rejectedWith(error).and.notify(done);
  });
});
