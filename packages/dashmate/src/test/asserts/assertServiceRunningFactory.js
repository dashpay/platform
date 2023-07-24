/**
 * @param {ConfigFile} configFile
 * @param {DockerCompose} dockerCompose
 *
 * @returns {assertServiceRunning}
 */
function assertServiceRunningFactory(configFile, dockerCompose) {
  /**
   * Check if service is running
   *
   * @typedef {assertServiceRunning}
   * @param {Config} config
   * @param {string} serviceName
   * @param {boolean} [expected=true]
   */
  async function assertServiceRunning(config, serviceName, expected = true) {
    const isRunning = await dockerCompose.isServiceRunning(
      config,
      serviceName,
    );

    let message = `Service ${serviceName} is expected to be running`;
    if (!expected) {
      message = `Service ${serviceName} is NOT expected to be running`;
    }

    expect(isRunning).to.equal(expected, message);
  }

  return assertServiceRunning;
}

module.exports = assertServiceRunningFactory;
