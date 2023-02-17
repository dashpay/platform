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
    } else if (preset === 'testnet' || preset === 'mainnet') {
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
    execute(`dashmate setup local --node-count=${nodes} --debug-logs --miner-interval=${minerInterval} ${args} --verbose`).then((res) => {
      if (res.status !== undefined) {
        throw new Error(`${res.stderr} with exit code: ${res.status}`);
      }
    });
  }

  /**
   * Setup testnet node
   * @param {string} nodeType - masternode, fullnode
   * @param {string} args
   */
  async setupTestnet(nodeType, ...args) {
    execute(`yarn dashmate setup testnet ${nodeType} ${args} --verbose`).then((res) => {
      if (res.status !== undefined) {
        throw new Error(`${res.stderr} with exit code: ${res.status}`);
      }
    });
  }

  /**
   * Start node
   * @param {string} preset - local, testnet, mainnet
   * @param {string} args
   */
  async start(preset, ...args) {
    const group = this.setNetwork(preset);

    execute(`yarn dashmate ${group} start ${args} --verbose`).then((res) => {
      if (res.status !== undefined) {
        throw new Error(`${res.stderr} with exit code: ${res.status}`);
      }
    });
  };

  /**
   * Stop node
   * @param {string} preset - local, testnet, mainnet
   * @param {string} args
   */
  async stop(preset, ...args) {
    const group = this.setNetwork(preset);

    execute(`yarn dashmate ${group} stop ${args} --verbose`).then((res) => {
      if (res.status !== undefined) {
        throw new Error(`${res.stderr} with exit code: ${res.status}`);
      }
    });
  };

  /**
   * Reset node
   * @param {string} preset - local, testnet, mainnet
   * @param {string} args
   */
  async reset(preset, ...args) {
    const group = this.setNetwork(preset);

    execute(`yarn dashmate ${group} reset ${args} --verbose`).then((res) => {
      if (res.status !== undefined) {
        throw new Error(`${res.stderr} with exit code: ${res.status}`);
      }
    });
  };

  /**
   * Restart node
   * @param {string} preset - local, testnet, mainnet
   * @param {string} args
   */
  async restart(preset, ...args) {
    const group = this.setNetwork(preset);

    execute(`yarn dashmate ${group} restart ${args} --verbose`).then((res) => {
      if (res.status !== undefined) {
        throw new Error(`${res.stderr} with exit code: ${res.status}`);
      }
    });
  };

  /**
   * Get service status
   * @param {string} service - '', core, host, masternode, platform, services
   * @param {boolean} allowErr - define if throw or return an error
   * @return {Promise<string>}
   */
  async checkStatus(service, allowErr = false) {
    return execute(`yarn dashmate status ${service}`).then((res) => {
      if (!allowErr) {
        if (res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`);
        } else {
          return res.toString();
        }
      } else {
        return res.toString();
      }
    });
  };

  /**
   * Get local status
   * @param {string} command - status, list
   * @param {boolean} allowErr - define if throw or return an error
   * @return {Promise<string>}
   */
  async checkGroupStatus(command, allowErr = false) {
    return execute(`yarn dashmate group ${command}`).then((res) => {
      if (!allowErr) {
        if (res.status !== undefined) {
          throw new Error(`${res.stderr} with exit code: ${res.status}`);
        } else {
          return res.toString();
        }
      } else {
        return res.toString();
      }
    });
  };
}

module.exports = TestDashmateClass;
