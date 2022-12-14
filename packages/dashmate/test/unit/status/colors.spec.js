const chalk = require('chalk');
const colors = require('../../../src/status/colors');
const ServiceStatusEnum = require('../../../src/enums/serviceStatus');

describe('colors.js', () => {
  describe('#portState', () => {
    it('should color green', async () => {
      const color = colors.portState('OPEN');

      expect(color).to.be.equal(chalk.green);
    });

    it('should color red', async () => {
      expect(colors.portState('CLOSED')).to.be.equal(chalk.red);
      expect(colors.portState(null)).to.be.equal(chalk.red);
      expect(colors.portState('any other stuff')).to.be.equal(chalk.red);
    });
  });

  describe('#status', () => {
    it('should color green', async () => {
      expect(colors.status(ServiceStatusEnum.up)).to.be.equal(chalk.green);
    });
    it('should color yellow', async () => {
      expect(colors.status(ServiceStatusEnum.syncing)).to.be.equal(chalk.yellow);
      expect(colors.status(ServiceStatusEnum.wait_for_core)).to.be.equal(chalk.yellow);
    });
    it('should color red', async () => {
      expect(colors.status(ServiceStatusEnum.error)).to.be.equal(chalk.red);
    });
  });

  describe('#version', () => {
    it('should color green', async () => {
      expect(colors.version('0.17.0.3', '0.17.0.3')).to.be.equal(chalk.green);
    });

    it('should color yellow', async () => {
      expect(colors.version('0.17.0.3', '0.17.0.4')).to.be.equal(chalk.yellow);
      expect(colors.version('0.17.0.3', '0.17.1.4')).to.be.equal(chalk.yellow);
      expect(colors.version('0.17.0.3', null)).to.be.equal(chalk.yellow);
    });

    it('should color red', async () => {
      expect(colors.version('0.16.0.3', '0.17.0.3')).to.be.equal(chalk.red);
    });
  });

  describe('#blockHeight', () => {
    it('should color green', async () => {
      expect(colors.blockHeight(1337, 1337)).to.be.equal(chalk.green);
      expect(colors.blockHeight(1337, 1337, 1336)).to.be.equal(chalk.green);
    });

    it('should color yellow', async () => {
      expect(colors.blockHeight(1336, 1337)).to.be.equal(chalk.yellow);
      expect(colors.blockHeight(1337, 1337, 1338)).to.be.equal(chalk.yellow);
      expect(colors.blockHeight(1337, 1337, 1339)).to.be.equal(chalk.yellow);
    });

    it('should color red', async () => {
      expect(colors.blockHeight(100, 1337, 1500)).to.be.equal(chalk.red);
      expect(colors.blockHeight(100, 1337)).to.be.equal(chalk.red);
    });
  });

  describe('#poSePenalty', () => {
    it('should color green', async () => {
      expect(colors.poSePenalty(0)).to.be.equal(chalk.green);
    });

    it('should color yellow', async () => {
      expect(colors.poSePenalty(2, 10)).to.be.equal(chalk.yellow);
    });

    it('should color red', async () => {
      expect(colors.poSePenalty(20, 2)).to.be.equal(chalk.red);
    });
  });
});
