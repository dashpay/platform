const { execute } = require('./runCommandInCli');

class TestDashmateClass {
  /**
   * Set preset whether it should be a single or group of nodes
   * @param {string} preset - local, testnet, mainnet
   * @return {string}
   */
  setNetwork(preset) {
    if (preset === 'local') {
      return 'group';
    } if (preset === 'testnet' || preset === 'mainnet') {
      return '';
    }

    throw new Error('Incorrect node configuration preset.');
  }

  /**
   * Setup local group of nodes
   * @param {number} nodes
   * @param {string} minerInterval
   * @param {string} args
   */
  async setupLocal(nodes = 3, minerInterval = '2.5m', ...args) {
    const res = await execute(`dashmate setup local --node-count=${nodes} --debug-logs --miner-interval=${minerInterval} ${args} --verbose`);

    if (res.status !== undefined) {
      throw new Error(`${res.stderr} with exit code: ${res.status}`);
    }
  }

  /**
   * Setup testnet node
   * @param {string} nodeType - masternode, fullnode
   * @param {string} args
   */
  async setupTestnet(nodeType, ...args) {
    const res = await execute(`yarn dashmate setup testnet ${nodeType} ${args} --verbose`);

    if (res.status !== undefined) {
      throw new Error(`${res.stderr} with exit code: ${res.status}`);
    }
  }

  /**
   * Start node
   * @param {string} preset - local, testnet, mainnet
   * @param {string} args
   */
  async start(preset, ...args) {
    const group = this.setNetwork(preset);

    const res = await execute(`yarn dashmate ${group} start ${args} --verbose`);

    if (res.status !== undefined) {
      throw new Error(`${res.stderr} with exit code: ${res.status}`);
    }
  }

  /**
   * Stop node
   * @param {string} preset - local, testnet, mainnet
   * @param {string} args
   */
  async stop(preset, ...args) {
    const group = this.setNetwork(preset);

    const res = await execute(`yarn dashmate ${group} stop ${args} --verbose`);

    if (res.status !== undefined) {
      throw new Error(`${res.stderr} with exit code: ${res.status}`);
    }
  }

  /**
   * Reset node
   * @param {string} preset - local, testnet, mainnet
   * @param {string} args
   */
  async reset(preset, ...args) {
    const group = this.setNetwork(preset);

    const res = await execute(`yarn dashmate ${group} reset ${args} --verbose`);

    if (res.status !== undefined) {
      throw new Error(`${res.stderr} with exit code: ${res.status}`);
    }
  }

  /**
   * Restart node
   * @param {string} preset - local, testnet, mainnet
   * @param {string} args
   */
  async restart(preset, ...args) {
    const group = this.setNetwork(preset);

    const res = await execute(`yarn dashmate ${group} restart ${args} --verbose`);

    if (res.status !== undefined) {
      throw new Error(`${res.stderr} with exit code: ${res.status}`);
    }
  }

  /**
   * Get single node status
   * @param {string} scope - '', core, host, masternode, platform, services
   * @return {Promise<string>}
   */
  async checkStatus(scope) {
    const res = await execute(`yarn dashmate status ${scope} --format=json`);

    if (res.status !== undefined) {
      throw new Error(`${res.stderr} with exit code: ${res.status}`);
    }

    return res.toString();
  }

  /**
   * Get group nodes status/list
   * @param {string} scope - status, list
   * @return {Promise<string>}
   */
  async getGroupStatus(scope) {
    const res = await execute(`yarn dashmate group ${scope} --format=json`);

    if (res.status !== undefined) {
      throw new Error(`${res.stderr} with exit code: ${res.status}`);
    }

    return res.toString();
  }
}

module.exports = TestDashmateClass;
