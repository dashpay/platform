const AbstractError = require('../../errors/AbstractError');

class DockerComposeError extends AbstractError {
  /**
   * @param {{err: string, out: string, exitCode: number}} dockerComposeExecutionResult
   */
  constructor(dockerComposeExecutionResult) {
    super(`Docker Compose error: ${dockerComposeExecutionResult.err || dockerComposeExecutionResult.message}`);

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
