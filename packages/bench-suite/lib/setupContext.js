const { convertCreditsToSatoshi } = require('@dashevo/dpp/lib/identity/creditsConverter');

const createClientWithFundedWallet = require('./client/createClientWithFundedWallet');

/**
 * @param {Mocha} mocha
 * @param {AbstractBenchmark[]} benchmarks
 * @param {Object} options
 */
function setupContext(mocha, benchmarks, options) {
  const context = mocha.suite.ctx;

  const requiredCredits = benchmarks.reduce(
    (sum, benchmark) => benchmark.getRequiredCredits() + sum,
    0,
  );

  let satoshis = convertCreditsToSatoshi(requiredCredits);

  if (satoshis < 10000) {
    satoshis = 10000;
  }

  mocha.suite.beforeAll('Create and connect client', async () => {
    context.dash = await createClientWithFundedWallet(
      satoshis + 5000,
      options.client,
    );
  });

  mocha.suite.beforeAll('Create identity', async () => {
    context.identity = await context.dash.platform.identities.register(satoshis);
  });

  mocha.suite.afterAll('Disconnect client', async () => {
    if (context.dash) {
      await context.dash.disconnect();
    }
  });
}

module.exports = setupContext;
