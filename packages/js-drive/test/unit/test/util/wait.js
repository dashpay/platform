const wait = require('../../../../lib/test/util/wait');

describe('wait', () => {
  it('should delay execution of a flow for a specified amount of milliseconds', async () => {
    const millisToWait = 1200;

    const startTime = process.hrtime();
    await wait(millisToWait);
    const endTime = process.hrtime(startTime);

    const endTimeMillis = (endTime[0] * 1e3) + (endTime[1] / 1e6);

    expect(endTimeMillis).to.be.at.least(millisToWait);
  });
});
