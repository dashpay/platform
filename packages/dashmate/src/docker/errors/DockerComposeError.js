const util = require('util');

const AbstractError = require('../../errors/AbstractError');

class DockerComposeError extends AbstractError {
  /**
   * @param {{err: string, out: string, exitCode: number}} dockerComposeExecutionResult
   */
  constructor(dockerComposeExecutionResult) {
    // TODO: Should be consistent
    const message = dockerComposeExecutionResult.err
      || dockerComposeExecutionResult.message
      || util.inspect(dockerComposeExecutionResult);

    super(`Docker Compose error: ${message}`);

    this.dockerComposeExecutionResult = dockerComposeExecutionResult;
  }

  /**
   * Get docker compose execution result
   *
   * @return {{err: string, out: string, exitCode: number}}
   */
  getDockerComposeResult() {
    return this.dockerComposeExecutionResult;
  }
}

module.exports = DockerComposeError;
