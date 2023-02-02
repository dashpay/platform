const { execute } = require('./runCommandInCli')

class TestDashmateClass {

  /**
   * Set preset whether it should be a single or group of nodes
   * @param {string} preset - local, testnet, mainnet
   * @return {string}
   */
  setNetwork(preset) {
    if(preset === 'local') {
      return 'group';
    } else if (preset === 'testnet' || preset === 'mainnet') {
      return '';
    } else {
      throw new Error('Incorrect node configuration preset.')
    }
  }

  /**
   * Setup local group of nodes
   * @param {number} nodes
   * @param {string} minerInterval
   * @param {string} args
   */
  async setupLocal(nodes = 3, minerInterval= '2.5m', ...args) {
    await execute(`dashmate setup local --node-count=${nodes} --debug-logs --miner-interval=${minerInterval} ${args} --verbose`).then(res => {
      if (res.status !== undefined) {
        throw new Error(`${res.stderr} with exit code: ${res.status}`)
      }
      // console.log(res.toString()); debug start logs
    })
  }

  /**
   * Setup testnet node
   * @param {string} nodeType - masternode, fullnode
   * @param {string} args
   */
  async setupTestnet(nodeType, ...args) {
    await execute(`yarn dashmate setup testnet ${nodeType} ${args} --verbose`).then(res => {
      if (res.status !== undefined) {
        throw new Error(`${res.stderr} with exit code: ${res.status}`)
      }
      console.log(res.toString()); //debug start logs
    })
  }

  /**
   * Start node
   * @param {string} preset - local, testnet, mainnet
   * @param {string} args
   */
  async start(preset, ...args) {
    const group = this.setNetwork(preset)

    await execute(`yarn dashmate ${group} start ${args} --verbose`).then(res => {
      if (res.status !== undefined) {
        throw new Error(`${res.stderr} with exit code: ${res.status}`)
      }
      // console.log(res.toString()); debug start logs
    })
  }

  /**
   * Stop node
   * @param {string} preset - local, testnet, mainnet
   * @param {string} args
   */
  async stop(preset, ...args) {
    const group = this.setNetwork(preset)

    await execute(`yarn dashmate ${group} stop ${args} --verbose`).then(res => {
      if (res.status !== undefined) {
        throw new Error(`${res.stderr} with exit code: ${res.status}`)
      }
      // console.log(res.toString()); debug start logs
    })
  }

  /**
   * Reset node
   * @param {string} preset - local, testnet, mainnet
   * @param {string} args
   */
  async reset(preset, ...args) {
    const group = this.setNetwork(preset)

    await execute(`yarn dashmate ${group} reset ${args} --verbose`).then(res => {
      if (res.status !== undefined) {
        throw new Error(`${res.stderr} with exit code: ${res.status}`)
      }
      // console.log(res.toString()); debug start logs
    })
  }

  /**
   * Restart node
   * @param {string} preset - local, testnet, mainnet
   * @param {string} args
   */
  async restart(preset, ...args) {
    const group = this.setNetwork(preset)

    await execute(`yarn dashmate ${group} restart ${args} --verbose`).then(res => {
      if (res.status !== undefined) {
        throw new Error(`${res.stderr} with exit code: ${res.status}`)
      }
      // console.log(res.toString()); debug start logs
    })
  }

  /**
   * Get service status
   * @param {string} service - '', core, host, masternode, platform, services
   * @return {Promise<string>}
   */
  async checkStatus(service) {
    return await execute(`yarn dashmate status ${service} --format=json`).then(res => {
      if (res.status !== undefined) {
        throw new Error(`${res.stderr} with exit code: ${res.status}`)
      }
      return res.toString();
    })
  }
}

module.exports = TestDashmateClass;
